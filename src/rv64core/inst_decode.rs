use alloc::vec::Vec;
use hashbrown::HashMap;

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

pub struct InstDecode {
    inst_vec: Vec<&'static Instruction>,
    inst_hash: HashMap<u32, &'static Instruction>,
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

        InstDecode {
            inst_vec: i_vec,
            inst_hash: HashMap::new(),
        }
    }

    fn hash_get(&self, inst_i: u32) -> Option<&&Instruction> {
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

    pub fn fast_path(&self, inst_i: u32) -> Option<&Instruction> {
        self.hash_get(inst_i).copied()
    }
}

impl Default for InstDecode {
    fn default() -> Self {
        Self::new()
    }
}
