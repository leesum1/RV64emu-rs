use alloc::vec::Vec;
use hashbrown::HashMap;
use hashlink::LruCache;
use log::info;

#[cfg(feature = "rv_a")]
use crate::rv64core::inst::inst_rv64a::INSTRUCTIONS_A;
#[cfg(feature = "rv_c")]
use crate::rv64core::inst::inst_rv64c::INSTRUCTIONS_C;
#[cfg(feature = "rv_m")]
use crate::rv64core::inst::inst_rv64m::INSTRUCTIONS_M;

use crate::rv64core::{
    inst::inst_base::Instruction, inst::inst_rv64i::INSTRUCTIONS_I,
    inst::inst_rv64z::INSTRUCTIONS_Z,
};

const DECODE_CACHE_SIZE: usize = 4096;

pub struct InstDecode {
    inst_vec: Vec<&'static Instruction>,
    inst_hash: LruCache<u32, &'static Instruction>,
    pub hit: u64,
    pub miss: u64,
    remove_count: u64,
}

impl InstDecode {
    pub fn new() -> Self {
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

        // let x = LruCache::new(8192);
        // let y = LinkedHashMap::new();

        InstDecode {
            inst_vec: i_vec,
            inst_hash: LruCache::new(DECODE_CACHE_SIZE),
            hit: 0,
            miss: 0,
            remove_count: 0,
        }
    }

    fn hash_get(&mut self, inst_i: u32) -> Option<&&Instruction> {
        // self.inst_hash.get(&inst_i)
        self.inst_hash.get(&inst_i)
    }

    pub fn slow_path(&mut self, inst_i: u32) -> Option<&Instruction> {
        let slowpath = self
            .inst_vec
            .iter()
            .find(|x| x.mask & inst_i == x.match_data)
            .copied();

        #[cfg(feature = "decode_cache")]
        if let Some(slow) = slowpath {
            self.inst_hash.insert(inst_i, slow);
        }
        slowpath
    }

    pub fn fast_path(&mut self, inst_i: u32) -> Option<&Instruction> {
        // self.hash_get(inst_i).copied()
        let ret = self.inst_hash.get(&inst_i).copied();
        if ret.is_some() {
            self.hit += 1;
        } else {
            self.miss += 1;
        }
        ret
    }

    // random remove a item from caches
    fn remove_random(&mut self) -> Option<&Instruction> {
        let (key, _) = self
            .inst_hash
            .iter()
            .next()
            .map(|(k, _)| (*k, ()))
            .unwrap_or_default();
        self.remove_count += 1;
        self.inst_hash.remove(&key)
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

impl Default for InstDecode {
    fn default() -> Self {
        Self::new()
    }
}
