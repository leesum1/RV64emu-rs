use hashbrown::HashMap;

use crate::{
    inst_base::Instruction, inst_rv64i::INSTRUCTIONS_I, inst_rv64m::INSTRUCTIONS_M,
    inst_rv64z::INSTRUCTIONS_Z,
};

pub struct InstDecode {
    inst_vec: Vec<&'static Instruction>,
    inst_hash: HashMap<u32, &'static Instruction>,
}

impl InstDecode {
    pub fn new() -> Self {
        let mut i_vec = Vec::new();
        i_vec.extend(&INSTRUCTIONS_I);
        i_vec.extend(&INSTRUCTIONS_M);
        i_vec.extend(&INSTRUCTIONS_Z);

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

        if let Some(slow) = slowpath {
            self.inst_hash.insert(inst_i, slow);
        }

        slowpath
    }

    pub fn fast_path(&self, inst_i: u32) -> Option<&Instruction> {
        self.hash_get(inst_i).copied()
    }
}
