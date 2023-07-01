use alloc::rc::Rc;

use crate::{rv64core::bus::Bus, tools::RcRefCell, config::Config};

use super::{cpu_dcache::CpuDcache, cpu_icache::CpuIcache};

pub struct CacheSystem {
    pub icache: CpuIcache,
    pub dcache: CpuDcache,
    pub bus: RcRefCell<Bus>,
    config: Rc<Config>,
}

impl CacheSystem {
    pub fn new(bus: RcRefCell<Bus>,config: Rc<Config>) -> Self {
        let icache = CpuIcache::new(bus.clone());
        let dcache = CpuDcache::new(bus.clone());
        CacheSystem {
            icache,
            dcache,
            bus,
            config,
        }
    }
    pub fn show_perf(&self) {
        self.icache.show_perf();
        self.dcache.show_perf();
    }

    pub fn clear(&mut self) {
        self.icache.clear();
        self.dcache.clear();
    }
}
