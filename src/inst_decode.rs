

use crate::{
    inst_base::Instruction, inst_rv64i::INSTRUCTIONS_I, inst_rv64m::INSTRUCTIONS_M,
    inst_rv64z::INSTRUCTIONS_Z,
};

pub struct InstDecode {
    inst_vec: Vec<&'static Instruction>,
}

impl InstDecode {
    pub fn new() -> Self {
        let mut i_vec = Vec::new();
        i_vec.extend(&INSTRUCTIONS_I);
        i_vec.extend(&INSTRUCTIONS_M);
        i_vec.extend(&INSTRUCTIONS_Z);

        InstDecode { inst_vec: i_vec }
    }

    pub fn step(&mut self, inst_i: u32) -> Option<&Instruction> {
        self.inst_vec
            .iter()
            .find(|x| x.mask & inst_i == x.match_data)
            .copied()
    }
}
