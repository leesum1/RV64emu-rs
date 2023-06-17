use hashbrown::HashMap;
use log::info;

use crate::rv64core::{bus::Bus, inst::inst_base::RVerr, traptype::RVmutex};

const ICACHE_SIZE: usize = 4096*10;

struct InstPack {
    inst: u32,
}
pub struct CpuIcache {
    bus: RVmutex<Bus>,
    inst_hash: HashMap<u64, InstPack>,
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
    #[cfg(feature = "inst_cache")]
    fn cacheble(&self, addr: u64) -> bool {
        (0x80000000..0x80000000 + 0x8000000).contains(&addr)
    }
    #[cfg(not(feature = "inst_cache"))]
    fn cacheble(&self, addr: u64) -> bool {
        false
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
                if self.inst_hash.len() >= ICACHE_SIZE {
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
