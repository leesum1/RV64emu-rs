use std::fmt::Debug;

use crate::{
    cpu_core::CpuCore, inst_base::Instruction, inst_rv64i::INSTRUCTIONS_I,
    inst_rv64m::INSTRUCTIONS_M, inst_rv64z::INSTRUCTIONS_Z,
};

pub struct InstDecode {
    inst_vec: Vec<Instruction>,
}

impl InstDecode {
    pub fn new() -> Self {
        let mut i_vec = Vec::new();
        i_vec.extend(INSTRUCTIONS_I);
        i_vec.extend(INSTRUCTIONS_M);
        i_vec.extend(INSTRUCTIONS_Z);

        InstDecode { inst_vec: i_vec }
    }

    pub fn step(&mut self, inst_i: u32) -> Option<&Instruction> {
        for element in self.inst_vec.iter() {
            if element.mask & inst_i == element.match_data {
                return Some(element);
            }
        }

        println!("inst:{inst_i:x}");
        panic!();
        None
    }
}
