use std::thread;
use std::time::Duration;

use crate::db;

pub(crate) struct Updater {
    pool: db::Pool,
    interval: Duration,
    // todo: price update send channel
}

impl Updater {
    pub fn new(pool: db::Pool, interval: Duration) -> Self {
        Updater { pool, interval }
    }

    pub fn start(&self) {
        let interval = self.interval.clone();
        let _pool = self.pool.clone();
        thread::spawn(move || loop {
            debug!("price update run");
            debug!("connections: {}", _pool.state().connections);
            thread::sleep(interval);
        });
    }
}
