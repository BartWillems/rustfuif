use std::{
    fmt,
    time::{Duration, Instant},
};

use actix::Addr;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};

use crate::errors::ServiceError;
use crate::games::Game;
use crate::websocket::server::{GameId, NotificationServer, PriceUpdate};
use crate::websocket::Notification;
use crate::{config::Config, games::Beverage};

#[must_use = "this `MarketStatus` may be a `Crash` variant, which should be handled"]
#[derive(Serialize, Debug, Copy, Clone)]
pub enum MarketStatus {
    Regular,
    Crash,
}

/// holds the current state of the stock market
///
/// when the stock market is crashed, all beverages will be
/// set to their lowest price
#[derive(Debug)]
pub(crate) struct StockMarket {
    last_crash: Instant,
    status: MarketStatus,
}

impl StockMarket {
    pub(crate) fn new() -> Self {
        StockMarket {
            // Make sure the market doesn't instantly crash
            last_crash: Instant::now(),
            status: MarketStatus::Regular,
        }
    }

    /// instantly crash the stockmarket
    /// this should only be used by administrators
    fn crash(&mut self) {
        // let mut inner = self.inner.write().await;
        self.last_crash = Instant::now();
        self.status = MarketStatus::Crash;
    }

    /// Set the market status to regular
    /// Should be used after a crash
    fn restore_market(&mut self) {
        // let mut inner = self.inner.write().await;
        self.status = MarketStatus::Regular;
    }

    /// returns true if the last stock market crash was at least
    /// 20 minutes ago
    pub(crate) fn can_crash(&self) -> bool {
        // let inner = self.inner.read().await;
        debug!(
            "Last crash: {} seconds ago",
            self.last_crash.elapsed().as_secs()
        );
        self.last_crash.elapsed().as_secs() > Config::market_crash_interval()
    }

    /// crash the stock market if it has been a while since the last crash
    ///
    /// Returns `true` if it has crashed
    pub(crate) fn update(&mut self) -> MarketStatus {
        if self.can_crash() {
            self.crash();
        } else {
            self.restore_market();
        }
        self.status
    }

    /// Returns true if a market crash is happening
    pub(crate) async fn has_crashed(&self) -> bool {
        matches!(self.status, MarketStatus::Crash)
    }
}

pub struct MarketAgent {
    db: Pool<Postgres>,
    notifier: Addr<NotificationServer>,
    market: StockMarket,
    game: Game,
}

impl fmt::Debug for MarketAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MarketAgent")
            .field("game", &self.game.id)
            .finish()
    }
}

impl MarketAgent {
    pub fn new(db: Pool<Postgres>, notifier: Addr<NotificationServer>, game: Game) -> Self {
        Self {
            db,
            notifier,
            market: StockMarket::new(),
            game,
        }
    }

    /// Start a periodic price updater
    pub(crate) fn start(mut self) {
        tokio::spawn(async move {
            debug!("Starting market agent for Game({})", self.game.id);
            if self.game.not_started() {
                actix_rt::time::delay_for(self.game.duration_until_start()).await;
            }

            while self.game.in_progress() {
                if self.game.is_finished() {
                    debug!("Game({}) is finished", self.game.id);
                    break;
                }
                actix_rt::time::delay_for(MarketAgent::interval()).await;

                self.update().await;
            }
        });
    }

    /// Update the prices and notify the users
    #[tracing::instrument(name = "StockMarket::update")]
    pub(crate) async fn update(&mut self) {
        match self.update_prices().await {
            Err(e) => {
                error!("unable to update prices: {}", e);
            }
            Ok(MarketStatus::Regular) => {
                debug!("succesfully updated the prices");
                self.notifier
                    .do_send(Notification::PriceUpdate(PriceUpdate {
                        market_status: MarketStatus::Regular,
                        game_id: GameId(self.game.id),
                    }));
            }
            Ok(MarketStatus::Crash) => {
                info!("succesfully updated the prices, with stock market crash");
                self.notifier
                    .do_send(Notification::PriceUpdate(PriceUpdate {
                        market_status: MarketStatus::Crash,
                        game_id: GameId(self.game.id),
                    }));
            }
        };
    }

    #[tracing::instrument(skip(self))]
    async fn update_prices(&mut self) -> Result<MarketStatus, ServiceError> {
        let start = Instant::now();

        let market_status = self.market.update();
        info!("Stock Market Status: {:?}", market_status);

        let mut tx = self.db.begin().await?;

        let beverages = match market_status {
            MarketStatus::Crash => self.game.crash_prices(&mut tx).await?,
            MarketStatus::Regular => self.game.update_prices(&mut tx).await?,
        };

        let changes: Vec<PriceChange> = beverages.iter().map(|beverage| beverage.into()).collect();

        PriceHistory::save(&changes, &mut tx).await?;

        tx.commit().await?;
        info!("updated game({}) in {:?}", self.game.id, start.elapsed());

        if self.market.has_crashed().await {
            return Ok(MarketStatus::Crash);
        }

        Ok(MarketStatus::Regular)
    }

    /// Retrieve the current price update interval
    pub(crate) fn interval() -> Duration {
        Duration::from_secs(Config::price_update_interval())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceHistory {
    id: i64,
    game_id: i64,
    user_id: i64,
    slot_no: i16,
    price: i64,
    created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub(crate) struct PriceChange {
    game_id: i64,
    user_id: i64,
    slot_no: i16,
    price: i64,
    created_at: DateTime<Utc>,
}

impl PriceHistory {
    /// Return all price changes for a single beverage
    #[tracing::instrument(name = "PriceHistory::load")]
    pub async fn load(
        user_id: i64,
        game_id: i64,
        db: &Pool<Postgres>,
    ) -> Result<Vec<PriceHistory>, sqlx::Error> {
        sqlx::query_as!(
            PriceHistory,
            "SELECT * FROM price_histories WHERE user_id = $1 AND game_id = $2",
            user_id,
            game_id
        )
        .fetch_all(db)
        .await
    }

    #[tracing::instrument(name = "PriceHistory::save", skip(db))]
    async fn save(
        changes: &[PriceChange],
        db: &mut sqlx::Transaction<'_, Postgres>,
    ) -> Result<(), sqlx::Error> {
        for change in changes {
            sqlx::query!(
                "INSERT INTO price_histories (game_id, user_id, slot_no, price, created_at) VALUES ($1, $2, $3, $4, $5)", 
                change.game_id, change.user_id, change.slot_no, change.price, change.created_at
            ).execute(&mut *db).await?;
        }
        Ok(())
    }
}

impl From<&Beverage> for PriceChange {
    fn from(beverage: &Beverage) -> Self {
        PriceChange {
            game_id: beverage.game_id,
            user_id: beverage.user_id,
            slot_no: beverage.slot_no,
            price: beverage.price(),
            created_at: Utc::now(),
        }
    }
}
