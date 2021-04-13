use std::time::Duration;

use actix::Addr;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};

use crate::errors::ServiceError;
use crate::games::Game;
use crate::market;
use crate::websocket::server::NotificationServer;
use crate::websocket::Notification;
use crate::{config::Config, games::Beverage};

#[derive(Serialize, Debug, Clone)]
pub enum PriceUpdate {
    Regular,
    /// When the stock market has crashed, all prices are set to their lowest
    /// possible value.
    StockMarketCrash,
}
pub(crate) struct Updater {
    db: Pool<Postgres>,
    interval: Duration,
    notifier: Addr<NotificationServer>,
}

impl Updater {
    pub fn new(db: Pool<Postgres>, notifier: Addr<NotificationServer>) -> Self {
        Updater {
            db,
            interval: Config::price_update_interval(),
            notifier,
        }
    }

    pub async fn start(self) {
        actix::spawn(async move {
            let mut stock_market = market::StockMarket::new();

            loop {
                actix_rt::time::delay_for(self.interval).await;

                match Updater::update_prices(&self.db, &mut stock_market).await {
                    Err(e) => {
                        error!("unable to update prices: {}", e);
                    }
                    Ok(PriceUpdate::Regular) => {
                        info!("succesfully updated the prices");
                        self.notifier
                            .do_send(Notification::PriceUpdate(PriceUpdate::Regular));
                    }
                    Ok(PriceUpdate::StockMarketCrash) => {
                        info!("succesfully updated the prices, with stock market crash");
                        self.notifier
                            .do_send(Notification::PriceUpdate(PriceUpdate::StockMarketCrash));
                    }
                };
            }
        });
    }

    #[tracing::instrument(skip(stock_market), name = "StockMarket::update_prices")]
    async fn update_prices(
        db: &Pool<Postgres>,
        stock_market: &mut market::StockMarket,
    ) -> Result<PriceUpdate, ServiceError> {
        let start = std::time::Instant::now();

        let should_crash = stock_market.maybe_crash();
        info!("has stockmarket crashed: {}", should_crash);

        let tx = db.begin().await?;

        let games = Game::active_games(db).await?;

        for game in &games {
            let beverages;
            if should_crash {
                beverages = game.crash_prices(db).await?;
            } else {
                beverages = game.update_prices(db).await?;
            }
            let changes: Vec<PriceChange> =
                beverages.iter().map(|beverage| beverage.into()).collect();

            PriceHistory::save(&changes, db).await?;
        }

        info!("updated {} games in {:?}", games.len(), start.elapsed());

        tx.commit().await?;

        if should_crash {
            return Ok(PriceUpdate::StockMarketCrash);
        }

        Ok(PriceUpdate::Regular)
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

    async fn save(changes: &[PriceChange], db: &Pool<Postgres>) -> Result<(), sqlx::Error> {
        for change in changes {
            sqlx::query!(
                "INSERT INTO price_histories (game_id, user_id, slot_no, price, created_at) VALUES ($1, $2, $3, $4, $5)", 
                change.game_id, change.user_id, change.slot_no, change.price, change.created_at
            ).execute(db).await?;
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
