use std::sync::atomic::{AtomicU32, Ordering};

lazy_static! {
    static ref STATS: Stats = Stats::new();
}

#[derive(Serialize, Debug)]
pub struct Stats {
    cache_hits: AtomicU32,
    cache_misses: AtomicU32,
}

#[derive(Serialize, Debug)]
pub struct LoadedStats {
    pub cache_hits: u32,
    pub cache_misses: u32,
}

impl Stats {
    fn new() -> Stats {
        Stats {
            cache_hits: AtomicU32::new(0u32),
            cache_misses: AtomicU32::new(0u32),
        }
    }

    pub(crate) fn cache_hit() {
        STATS.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn cache_miss() {
        STATS.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Load the atomic stats variables as regular u32's
    pub fn load() -> LoadedStats {
        LoadedStats {
            cache_hits: STATS.cache_hits.load(Ordering::Relaxed),
            cache_misses: STATS.cache_misses.load(Ordering::Relaxed),
        }
    }
}
