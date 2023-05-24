use capstone::prelude::*;

use crate::rv64core::traptype::TrapType;
use std::fs::File;
use std::io::Write;

pub struct Itrace {
    cs_disasm: Capstone,
    log_file: File,
}
unsafe impl Send for Itrace {}

impl Itrace {
    pub fn new(hart_id: usize) -> Self {
        let cs_riscv = Capstone::new()
            .riscv()
            .mode(arch::riscv::ArchMode::RiscV64)
            .detail(false)
            .build()
            .unwrap();

        let path = format!("/tmp/rv64emu_itrace_logs_{}", hart_id);
        let fd = File::create(path).unwrap();

        Itrace {
            log_file: fd,
            cs_disasm: cs_riscv,
        }
    }

    pub fn disassemble_bytes(&mut self, pc: u64, inst: u32) {
        let insns = self.cs_disassemble_bytes(pc, inst);
        self.itrace_log(&insns);
    }
    fn cs_disassemble_bytes(&mut self, pc: u64, inst: u32) -> String {
        let code = inst.to_le_bytes();
        let insns = self.cs_disasm.disasm_all(&code, pc).unwrap();
        insns.to_string()
    }
    pub fn trap_record(&mut self, trap_type: TrapType, epc: u64, tval: u64) {
        self.itrace_log(&format!(
            "exception:{},epc:{:x},tval:{:x}\n",
            trap_type, epc, tval
        ));
    }

    fn itrace_log(&mut self, log: &str) {
        self.log_file.write_all(log.as_bytes()).unwrap();
    }
}
