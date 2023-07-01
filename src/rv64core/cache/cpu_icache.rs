use hashbrown::HashMap;
use log::info;

use crate::{
    rv64core::{bus::Bus, inst::inst_base::RVerr},
    tools::RcRefCell,
};

struct InstPack {
    inst: u32,
}
pub struct CpuIcache {
    bus: RcRefCell<Bus>,
    inst_hash: HashMap<u64, InstPack>,
    icache_size: usize,
    hit: u64,
    miss: u64,
}

impl CpuIcache {
    pub fn new(bus: RcRefCell<Bus>, size: usize) -> Self {
        CpuIcache {
            bus,
            inst_hash: HashMap::new(),
            icache_size: size,
            hit: 0,
            miss: 0,
        }
    }

    fn cacheble(&self, addr: u64) -> bool {
        if self.icache_size == 0 {
            false
        } else {
            (0x80000000..0x80000000 + 0x8000000).contains(&addr)
        }
    }
    // todo len:2,4
    pub fn read(&mut self, pc: u64, len: usize) -> Result<u64, RVerr> {
        let addr: u64 = pc;
        if !self.cacheble(addr) {
            let mut bus = self.bus.borrow_mut();
            return bus.read(addr, len);
        }

        if let Some(inst_pack) = self.inst_hash.get(&pc) {
            self.hit += 1;
            return Ok(inst_pack.inst as u64);
        }
        let mut bus = self.bus.borrow_mut();
        match bus.read(addr, len) {
            Ok(data) => {
                self.miss += 1;
                if self.inst_hash.len() >= self.icache_size {
                    drop(bus);
                    self.remove_random();
                }
                self.inst_hash.insert(pc, InstPack { inst: data as u32 });
                Ok(data)
            }
            Err(err) => Err(err),
        }
    }
    pub fn write(&mut self, _addr: u64, _data: u32) -> Result<(), RVerr> {
        Err(RVerr::NotFindDevice)
    }
    // random remove a item from caches
    fn remove_random(&mut self) -> Option<InstPack> {
        let (key, _) = self
            .inst_hash
            .iter()
            .next()
            .map(|(k, _)| (*k, ()))
            .unwrap_or_default();
        self.inst_hash.remove(&key)
    }

    pub fn clear(&mut self) {
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
