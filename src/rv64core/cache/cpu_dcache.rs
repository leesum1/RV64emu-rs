use crate::rv64core::bus::Bus;
use crate::rv64core::inst::inst_base::RVerr;
use crate::rv64core::traptype::RVmutex;
use log::info;

#[derive(Clone)]
struct CacheLine {
    valid: bool,
    dirty: bool,
    lru: u8,
    tag: u64,
    data: [u8; 64],
}

impl CacheLine {
    pub fn new() -> Self {
        CacheLine {
            valid: false,
            dirty: false,
            lru: 0,
            tag: 0,
            data: [0; 64],
        }
    }
    pub fn hit(&self, tag: u64) -> bool {
        self.valid && self.tag == tag
    }
    pub fn dirty(&self) -> bool {
        self.dirty
    }
    pub fn read(&mut self, addr: usize, len: usize) -> u64 {
        self.inc_lru();

        let mut des = 0_u64.to_le_bytes();
        des[..len].copy_from_slice(&self.data[addr..(len + addr)]);
        u64::from_le_bytes(des)
    }
    pub fn write(&mut self, addr: usize, data: u64, len: usize) {
        self.inc_lru();

        let src = data.to_le_bytes();
        self.data[addr..(len + addr)].copy_from_slice(&src[..len]);
        self.dirty = true;
    }
    fn inc_lru(&mut self) {
        self.lru = self.lru.wrapping_add(1);
        // self.lru += 1;
    }

    pub fn clear(&mut self) {
        self.valid = false;
        self.dirty = false;
        self.lru = 0;
    }
}

pub struct CpuDcache {
    bus: RVmutex<Bus>,
    // caches: Vec<CacheLine>,
    caches: hashbrown::HashMap<u64, CacheLine>,
    hit: u64,
    miss: u64,
}

impl CpuDcache {
    pub fn new(bus: RVmutex<Bus>) -> Self {
        // let caches = vec![CacheLine::new(); 32];
        let caches = hashbrown::HashMap::new();
        CpuDcache {
            bus,
            caches,
            hit: 0,
            miss: 0,
        }
    }
    fn tag(&self, addr: u64) -> u64 {
        addr >> 6
    }
    fn offset(&self, addr: u64) -> usize {
        (addr & 0x3f) as usize
    }
    fn cacheline_base(&self, addr: u64) -> u64 {
        addr & !0x3f
    }
    #[cfg(feature = "data_cache")]
    fn cacheble(&self, addr: u64) -> bool {
        (0x80000000..0x80000000 + 0x8000000).contains(&addr)
    }
    #[cfg(not(feature = "data_cache"))]
    fn cacheble(&self, addr: u64) -> bool {
        false
    }

    pub fn read(&mut self, addr: u64, len: usize) -> Result<u64, RVerr> {
        if !self.cacheble(addr) {
            let mut bus = self.bus.borrow_mut();
            return bus.read(addr, len);
        }

        let tag = self.tag(addr);
        let offset = self.offset(addr);

        self.caches
            .get_mut(&tag)
            .map(|cache_line| {
                self.hit += 1;
                Ok(cache_line.read(offset, len))
            })
            .unwrap_or_else(|| {
                self.miss += 1;
                self.alloc_cache_line(addr);
                self.read(addr, len)
            })

    }
    pub fn write(&mut self, addr: u64, data: u64, len: usize) -> Result<u64, RVerr> {
        if !self.cacheble(addr) {
            let mut bus = self.bus.borrow_mut();
            return bus.write(addr, data, len);
        }
        let tag = self.tag(addr);
        let offset = self.offset(addr);

        self.caches
            .get_mut(&tag)
            .map(|cache_line| {
                self.hit += 1;
                cache_line.write(offset, data, len);
                Ok(0)
            })
            .unwrap_or_else(|| {
                self.miss += 1;
                self.alloc_cache_line(addr);
                self.write(addr, data, len)
            })
    }
    pub fn clear(&mut self) {
        let mut bus = self.bus.borrow_mut();
        self.caches.iter_mut().for_each(|(_, cache_line)| {
            if cache_line.dirty() {
                let addr = cache_line.tag << 6;
                for i in (0..64).step_by(8) {
                    let data = cache_line.read(i, 8);
                    bus.write(addr + i as u64, data, 8).unwrap();
                }
            }
            // cache_line.clear()
        });
        self.caches.clear();
    }

    pub fn show_perf(&self) {
        info!("dcache hit: {}, miss: {}", self.hit, self.miss);
        info!(
            "dcache hit rate: {}",
            self.hit as f64 / (self.hit + self.miss) as f64
        )
    }

    fn alloc_cache_line(&mut self, addr: u64) {
        let tag = self.tag(addr);
        let cacheline_base = self.cacheline_base(addr);

        let mut bus = self.bus.borrow_mut();
        let mut cache_data = [0_u8; 64];
        for i in (0..64).step_by(8) {
            let data = bus.read(cacheline_base + i as u64, 8).unwrap();
            cache_data[i..(i + 8)].copy_from_slice(&data.to_le_bytes());
        }

        let new_line = CacheLine {
            valid: true,
            dirty: false,
            lru: 0,
            tag,
            data: cache_data,
        };

        self.caches.insert(tag, new_line);

        let need_write_back = false;
        // // todo! read from memory
        // let need_write_back = self
        //     .caches
        //     .iter_mut()
        //     .find(|(_, cache_line)| !cache_line.valid)
        //     .map(|(_, cache_line)| {
        //         cache_line.valid = true;
        //         cache_line.dirty = false;
        //         cache_line.lru = 0;
        //         cache_line.tag = tag;
        //         cache_line.data = cache_data;
        //     })
        //     .is_none();

        if need_write_back {
            panic!("123");
            let (_, cache_line) = self
                .caches
                .iter_mut()
                .min_by_key(|(_, cache_line)| cache_line.lru)
                .unwrap();

            let addr = cache_line.tag << 6;
            for i in (0..64).step_by(8) {
                let data = cache_line.read(i, 8);
                bus.write(addr + i as u64, data, 8).unwrap();
            }

            cache_line.valid = true;
            cache_line.dirty = false;
            cache_line.lru = 0;
            cache_line.tag = tag;
            cache_line.data = cache_data;
        };
    }

    // fn write_back(&mut self, cacheline: &mut CacheLine) {
    //     let mut bus = self.bus.borrow_mut();
    //     let addr = cacheline.tag << 6;
    //     for i in (0..64).step_by(8) {
    //         let data = cacheline.read(i, 8);
    //         bus.write(addr + i as u64, data, 8).unwrap();
    //     }
    // }
}

// impl Default for CpuDcache {
//     fn default() -> Self {
//         Self::new()
//     }
// }
