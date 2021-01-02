use std::sync::Arc;
use std::thread;
use std::time::Duration;

use actix::Addr;

use crate::config::Config;
use crate::db;
use crate::errors::ServiceError;
use crate::games::Game;
use crate::market;
use crate::websocket::server::NotificationServer;
use crate::websocket::Notification;

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
                if should_crash {
                    game.crash_prices(&conn)?;
                } else {
                    game.update_prices(&conn)?;
                }
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
