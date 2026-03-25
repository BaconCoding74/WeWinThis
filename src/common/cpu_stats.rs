use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration};

#[derive(Debug)]
pub struct CpuStats {
    active_ns: AtomicU64,
    idle_ns: AtomicU64,
}

impl CpuStats {
    pub const fn new() -> Self {
        Self {
            active_ns: AtomicU64::new(0),
            idle_ns: AtomicU64::new(0),
        }
    }

    pub fn add_active(&self, d: Duration) {
        self.active_ns
            .fetch_add(d.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn add_idle(&self, d: Duration) {
        self.idle_ns
            .fetch_add(d.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn active(&self) -> Duration {
        Duration::from_nanos(self.active_ns.load(Ordering::Relaxed))
    }

    pub fn idle(&self) -> Duration {
        Duration::from_nanos(self.idle_ns.load(Ordering::Relaxed))
    }

    pub fn utilization_pct(&self) -> f64 {
        let active = self.active_ns.load(Ordering::Relaxed) as f64;
        let idle = self.idle_ns.load(Ordering::Relaxed) as f64;
        let total = active + idle;

        if total <= 0.0 {
            0.0
        } else {
            (active / total) * 100.0
        }
    }
}

pub fn print_cpu_stats(stats: &CpuStats) {
    let active = stats.active();
    let idle = stats.idle();
    let total = active + idle;

    let util = if total.as_nanos() == 0 {
        0.0
    } else {
        (active.as_secs_f64() / total.as_secs_f64()) * 100.0
    };
    println!("active time : {:?}", active);
    println!("idle time   : {:?}", idle);
    println!("total time  : {:?}", total);
    println!("utilization : {:.2}%", util);
    println!();
}