use super::*;

pub struct LeakDetector {
baseline: Mutex<u64>,
growth_threshold_bytes: u64,
consecutive_growth_count: Mutex<usize>,
max_consecutive_growth: usize,
}

impl LeakDetector {
pub fn new(growth_threshold_mb: u64, max_consecutive_growth: usize) -> Self {
Self {
baseline: Mutex::new(0),
growth_threshold_bytes: growth_threshold_mb * 1024 * 1024,
consecutive_growth_count: Mutex::new(0),
max_consecutive_growth,
}
}


pub fn reset_baseline(&self) {
    let current = MemoryStats::current();
    if let Ok(mut baseline) = self.baseline.lock() {
        *baseline = current.rss_bytes;
    }
    if let Ok(mut count) = self.consecutive_growth_count.lock() {
        *count = 0;
    }
}

pub fn check(&self) -> Option<String> {
    let current = MemoryStats::current();

    let baseline_val = match self.baseline.lock() {
        Ok(b) => *b,
        Err(_) => return None,
    };

    if baseline_val == 0 {
        if let Ok(mut baseline) = self.baseline.lock() {
            *baseline = current.rss_bytes;
        }
        return None;
    }

    let growth = current.rss_bytes.saturating_sub(baseline_val);

    if growth > self.growth_threshold_bytes {
        let count = match self.consecutive_growth_count.lock() {
            Ok(mut c) => {
                *c += 1;
                *c
            }
            Err(_) => return None,
        };

        if count >= self.max_consecutive_growth {
            return Some(format!(
                "POTENTIAL MEMORY LEAK: grew by {} over {} checks. RSS={}, Baseline={}",
                MemoryStats::format_bytes(growth),
                count,
                MemoryStats::format_bytes(current.rss_bytes),
                MemoryStats::format_bytes(baseline_val),
            ));
        }
    } else {
        if let Ok(mut count) = self.consecutive_growth_count.lock() {
            *count = 0;
        }
        if let Ok(mut baseline) = self.baseline.lock() {
            *baseline = current.rss_bytes;
        }
    }

    None
}

}
