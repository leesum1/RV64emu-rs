

use hashbrown::HashMap;
use log::info;

use crate::rv64core::{bus::Bus, inst::inst_base::RVerr, traptype::RVmutex};

pub struct CpuIcache {
    bus: RVmutex<Bus>,
    inst_hash: HashMap<u64, u32>,
    hit: u64,
    miss: u64,
}

impl CpuIcache {
    pub fn new(bus: RVmutex<Bus>) -> Self {
        CpuIcache {
            bus,
            inst_hash: HashMap::new(),
            hit: 0,
            miss: 0,
        }
    }

    pub fn read(&mut self, pc: u64) -> Result<u64, RVerr> {
        let addr = pc;
        if let Some(inst) = self.inst_hash.get(&pc) {
            self.hit += 1;
            return Ok(*inst as u64);
        }
        let mut bus = self.bus.lock();
        match bus.read(addr, 4) {
            Ok(data) => {
                self.miss += 1;
                self.inst_hash.insert(pc, data as u32);
                Ok(data)
            }
            Err(err) => Err(err),
        }
    }
    pub fn write(&mut self, _addr: u64, _data: u32) -> Result<(), RVerr> {
        Err(RVerr::NotFindDevice)
    }

    pub fn clear_inst(&mut self) {
        self.inst_hash.clear();
    }
    pub fn show_perf(&self) {
        info!("icache hit: {}, miss: {}", self.hit, self.miss);
        info!(
            "icache hit rate: {:.2}%",
            self.hit as f64 / (self.hit + self.miss) as f64 * 100.0
        );
    }
}
