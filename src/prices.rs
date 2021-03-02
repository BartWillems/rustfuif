use std::sync::Arc;
use std::thread;
use std::time::Duration;

use actix::Addr;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db;
use crate::errors::ServiceError;
use crate::games::Game;
use crate::market;
use crate::schema::price_histories;
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
    pool: db::Pool,
    interval: Duration,
    notifier: Arc<Addr<NotificationServer>>,
}

impl Updater {
    pub fn new(pool: db::Pool, notifier: Arc<Addr<NotificationServer>>) -> Self {
        Updater {
            pool,
            interval: Config::price_update_interval(),
            notifier,
        }
    }

    pub fn start(&self) {
        let interval = self.interval;
        let pool = self.pool.clone();
        let notifier = self.notifier.clone();

        thread::spawn(move || {
            let mut stock_market = market::StockMarket::new();

            loop {
                thread::sleep(interval);

                match Updater::update_prices(pool.clone(), &mut stock_market) {
                    Err(e) => {
                        error!("unable to update prices: {}", e);
                    }
                    Ok(PriceUpdate::Regular) => {
                        info!("succesfully updated the prices");
                        notifier.do_send(Notification::PriceUpdate(PriceUpdate::Regular));
                    }
                    Ok(PriceUpdate::StockMarketCrash) => {
                        info!("succesfully updated the prices, with stock market crash");
                        notifier.do_send(Notification::PriceUpdate(PriceUpdate::StockMarketCrash));
                    }
                };
            }
        });
    }

    #[tracing::instrument(skip(pool, stock_market))]
    fn update_prices(
        pool: db::Pool,
        stock_market: &mut market::StockMarket,
    ) -> Result<PriceUpdate, ServiceError> {
        use diesel::prelude::*;
        let start = std::time::Instant::now();
        let conn = pool.get()?;

        let should_crash = stock_market.maybe_crash();
        info!("has stockmarket crashed: {}", should_crash);

        conn.transaction::<(), ServiceError, _>(|| {
            let games = Game::active_games(&conn)?;

            for game in &games {
                let beverages;
                if should_crash {
                    beverages = game.crash_prices(&conn)?;
                } else {
                    beverages = game.update_prices(&conn)?;
                }

                let changes: Vec<PriceChange> =
                    beverages.iter().map(|beverage| beverage.into()).collect();

                PriceHistory::save(&changes, &conn)?;
            }
            info!("updated {} games in {:?}", games.len(), start.elapsed());

            Ok(())
        })?;

        if should_crash {
            return Ok(PriceUpdate::StockMarketCrash);
        }

        Ok(PriceUpdate::Regular)
    }
}

#[derive(Debug, Serialize, Queryable, Identifiable)]
#[table_name = "price_histories"]
#[serde(rename_all = "camelCase")]
pub struct PriceHistory {
    id: i64,
    game_id: i64,
    user_id: i64,
    slot_no: i16,
    price: i64,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[table_name = "price_histories"]
pub(crate) struct PriceChange {
    game_id: i64,
    user_id: i64,
    slot_no: i16,
    price: i64,
    created_at: DateTime<Utc>,
}

impl PriceHistory {
    /// Return all price changes for a single beverage
    pub fn load(
        user_id: i64,
        game_id: i64,
        conn: &db::Conn,
    ) -> Result<Vec<PriceHistory>, diesel::result::Error> {
        price_histories::table
            .filter(price_histories::user_id.eq(user_id))
            .filter(price_histories::game_id.eq(game_id))
            .load(conn)
    }

    fn save(
        changes: &[PriceChange],
        conn: &db::Conn,
    ) -> Result<PriceHistory, diesel::result::Error> {
        diesel::insert_into(price_histories::table)
            .values(changes)
            .get_result(conn)
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
