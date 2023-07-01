use alloc::{rc::Rc, vec::Vec};
use hashlink::LruCache;
use log::info;

#[cfg(feature = "rv_a")]
use crate::rv64core::inst::inst_rv64a::INSTRUCTIONS_A;
#[cfg(feature = "rv_c")]
use crate::rv64core::inst::inst_rv64c::INSTRUCTIONS_C;
#[cfg(feature = "rv_m")]
use crate::rv64core::inst::inst_rv64m::INSTRUCTIONS_M;

use crate::{
    config::Config,
    rv64core::{
        inst::inst_base::Instruction, inst::inst_rv64i::INSTRUCTIONS_I,
        inst::inst_rv64z::INSTRUCTIONS_Z,
    },
};

pub struct InstDecode {
    inst_vec: Vec<&'static Instruction>,
    inst_hash: LruCache<u32, &'static Instruction>,
    pub hit: u64,
    pub miss: u64,
    remove_count: u64,
    config: Rc<Config>,
}

impl InstDecode {
    pub fn new(config: Rc<Config>) -> Self {
        let mut i_vec = Vec::new();
        i_vec.extend(INSTRUCTIONS_I);
        i_vec.extend(INSTRUCTIONS_Z);
        #[cfg(feature = "rv_m")]
        i_vec.extend(INSTRUCTIONS_M);
        #[cfg(feature = "rv_a")]
        i_vec.extend(INSTRUCTIONS_A);
        #[cfg(feature = "rv_c")]
        i_vec.extend(INSTRUCTIONS_C);

        i_vec.sort_by(|a: &&Instruction, b: &&Instruction| Instruction::inst_cmp(a, b));

        InstDecode {
            inst_vec: i_vec,
            inst_hash: LruCache::new(config.decode_cache_size().unwrap_or(0)),
            hit: 0,
            miss: 0,
            remove_count: 0,
            config,
        }
    }

    fn no_decode_cache(&self) -> bool {
        self.inst_hash.capacity() == 0
    }

    fn slow_path(&mut self, inst_i: u32) -> Option<&Instruction> {
        let slowpath = self
            .inst_vec
            .iter()
            .find(|x| x.mask & inst_i == x.match_data)
            .copied();

        if !self.no_decode_cache() {
            if let Some(slow) = slowpath {
                self.inst_hash.insert(inst_i, slow);
            }
        }

        slowpath
    }

    pub fn fast_path(&mut self, inst_i: u32) -> Option<&Instruction> {
        if self.no_decode_cache() {
            return self.slow_path(inst_i);
        }

        let ret = self.inst_hash.get(&inst_i).copied();
        if ret.is_some() {
            self.hit += 1;
            ret
        } else {
            self.miss += 1;
            self.slow_path(inst_i)
        }
    }

    pub fn show_perf(&self) {
        info!(
            "decode cache hit: {}, miss: {}, remove: {}",
            self.hit, self.miss, self.remove_count
        );
        info!(
            "decode cache hit rate: {:.2}%",
            self.hit as f64 / (self.hit + self.miss) as f64 * 100.0
        )
    }
}
