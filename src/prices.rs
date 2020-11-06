use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::db;
use crate::errors::ServiceError;
use crate::games::Game;
use crate::websocket::Notification;

pub(crate) struct Updater {
    pool: db::Pool,
    interval: Duration,
    notifier: mpsc::Sender<Notification>,
}

impl Updater {
    pub fn new(pool: db::Pool, interval: Duration, notifier: mpsc::Sender<Notification>) -> Self {
        Updater {
            pool,
            interval,
            notifier,
        }
    }

    pub fn start(&self) {
        let interval = self.interval;
        let pool = self.pool.clone();
        let notifier = self.notifier.clone();
        thread::spawn(move || loop {
            thread::sleep(interval);
            match Updater::update_prices(pool.clone()) {
                Err(e) => {
                    error!("unable to update prices: {}", e);
                }
                Ok(()) => {
                    info!("succesfully updated the prices");
                    if let Err(err) = notifier.send(Notification::PriceUpdate) {
                        error!("unable to notify users about price update: {}", err);
                    }
                }
            };
        });
    }

    fn update_prices(pool: db::Pool) -> Result<(), ServiceError> {
        use diesel::prelude::*;
        let start = std::time::Instant::now();
        let conn = pool.get()?;
        conn.transaction::<(), ServiceError, _>(|| {
            let games = Game::active_games(&conn)?;

            for game in &games {
                game.update_prices(&conn)?;
            }
            debug!("updated {} games in {:?}", games.len(), start.elapsed());

            Ok(())
        })?;

        Ok(())
    }
}
