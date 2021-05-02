use std::time::Instant;

#[must_use = "this `MarketStatus` may be a `Crash` variant, which should be handled"]
#[derive(Debug, Copy, Clone)]
pub(crate) enum MarketStatus {
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

    /// returns true if the last stock market crash was at least
    /// 20 minutes ago
    pub(crate) fn can_crash(&self) -> bool {
        self.last_crash.elapsed().as_secs() > 60 * 20
    }

    /// crash the stock market if it has been a while since the last crash
    ///
    /// Returns `true` if it has crashed
    pub(crate) fn update(&mut self) -> MarketStatus {
        if self.can_crash() {
            self.crash();
        }
        self.status
    }

    /// Returns true if a market crash is happening
    pub(crate) fn has_crashed(&self) -> bool {
        match self.status {
            MarketStatus::Crash => true,
            _ => false,
        }
    }
}
