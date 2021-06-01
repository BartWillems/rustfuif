use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use actix::Addr;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};
use tokio::sync::RwLock;

use crate::errors::ServiceError;
use crate::games::Game;
use crate::websocket::server::NotificationServer;
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
        self.last_crash = Instant::now();
        self.status = MarketStatus::Crash;
    }

    /// Set the market status to regular
    /// Should be used after a crash
    fn restore_market(&mut self) {
        self.status = MarketStatus::Regular;
    }

    /// returns true if the last stock market crash was at least
    /// 20 minutes ago
    pub(crate) fn can_crash(&self) -> bool {
        debug!(
            "Last crash: {} seconds ago",
            self.last_crash.elapsed().as_secs()
        );
        self.last_crash.elapsed().as_secs() > 60 * 20
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
    pub(crate) fn has_crashed(&self) -> bool {
        matches!(self.status, MarketStatus::Crash)
    }
}

#[derive(Clone)]
pub struct MarketAgent {
    db: Pool<Postgres>,
    interval: Arc<AtomicU64>,
    notifier: Addr<NotificationServer>,
    market: Arc<RwLock<StockMarket>>,
}

impl MarketAgent {
    pub fn new(db: Pool<Postgres>, notifier: Addr<NotificationServer>) -> Self {
        Self {
            db,
            interval: Arc::new(Config::price_update_interval()),
            notifier,
            market: Arc::new(RwLock::new(StockMarket::new())),
        }
    }

    /// Start a periodic price updater
    pub(crate) async fn start(&self) {
        let agent = self.clone();
        actix::spawn(async move {
            loop {
                actix_rt::time::delay_for(agent.interval()).await;

                agent.update().await;
            }
        });
    }

    /// Update the prices and notify the users
    pub(crate) async fn update(&self) {
        match self.update_prices().await {
            Err(e) => {
                error!("unable to update prices: {}", e);
            }
            Ok(MarketStatus::Regular) => {
                info!("succesfully updated the prices");
                self.notifier
                    .do_send(Notification::PriceUpdate(MarketStatus::Regular));
            }
            Ok(MarketStatus::Crash) => {
                info!("succesfully updated the prices, with stock market crash");
                self.notifier
                    .do_send(Notification::PriceUpdate(MarketStatus::Crash));
            }
        };
    }

    #[tracing::instrument(name = "StockMarket::update_prices", skip(self))]
    async fn update_prices(&self) -> Result<MarketStatus, ServiceError> {
        let start = Instant::now();

        // Acquire write lock
        let mut market = self.market.write().await;

        let market_status = market.update();
        info!("Stock Market Status: {:?}", market_status);

        let tx = self.db.begin().await?;

        let games = Game::active_games(&self.db).await?;

        for game in &games {
            let beverages = match market_status {
                MarketStatus::Crash => game.crash_prices(&self.db).await?,
                MarketStatus::Regular => game.update_prices(&self.db).await?,
            };

            let changes: Vec<PriceChange> =
                beverages.iter().map(|beverage| beverage.into()).collect();

            PriceHistory::save(&changes, &self.db).await?;
        }

        tx.commit().await?;
        info!("updated {} games in {:?}", games.len(), start.elapsed());

        if market.has_crashed() {
            return Ok(MarketStatus::Crash);
        }

        Ok(MarketStatus::Regular)
    }

    /// Overwrite the current price update interval
    /// This takes effect after 1 price update iteration
    pub(crate) async fn set_interval(&self, new_interval: u64) {
        self.interval.store(new_interval, Ordering::SeqCst);
    }

    /// Retrieve the current price update interval
    pub(crate) fn interval(&self) -> Duration {
        Duration::from_secs(self.interval.load(Ordering::SeqCst))
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
    async fn save(changes: &[PriceChange], db: &Pool<Postgres>) -> Result<(), sqlx::Error> {
        let futures: Vec<_> = changes.iter().map(|change| {
            sqlx::query!(
                "INSERT INTO price_histories (game_id, user_id, slot_no, price, created_at) VALUES ($1, $2, $3, $4, $5)", 
                change.game_id, change.user_id, change.slot_no, change.price, change.created_at
            ).execute(db)
        }).collect();

        futures::future::try_join_all(futures).await?;

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
