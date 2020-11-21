use std::time::Instant;

/// how many price updates a stock market crash takes
// const PRICE_ITER: u8 = 2;

/// holds the current state of the stock market
///
/// when the stock market is crashed, all beverages will be
/// set to their lowest price
pub(crate) struct StockMarket {
    /// the time when te crash started
    crash_instant: Instant,
}

impl StockMarket {
    pub(crate) fn new() -> Self {
        StockMarket {
            crash_instant: Instant::now(),
        }
    }

    /// instantly crash the stockmarket
    /// this should only be used by administrators
    fn crash(&mut self) {
        self.crash_instant = Instant::now();
    }

    /// returns true if the last stock market crash was at least
    /// 20 minutes ago
    pub(crate) fn can_crash(&self) -> bool {
        self.crash_instant.elapsed().as_secs() > 60 * 20
    }

    /// crash the stock market if it has been a while since the last crash
    ///
    /// Returns `true` if it has crashed
    pub(crate) fn maybe_crash(&mut self) -> bool {
        if self.can_crash() {
            self.crash();
            return true;
        }
        false
    }
}
