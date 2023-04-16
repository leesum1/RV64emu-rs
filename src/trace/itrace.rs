use llvm_sys::disassembler::{
    LLVMCreateDisasmCPUFeatures, LLVMDisasmInstruction, LLVMOpaqueDisasmContext,
};
use llvm_sys::target::{
    LLVMInitializeRISCVAsmParser, LLVMInitializeRISCVAsmPrinter, LLVMInitializeRISCVDisassembler,
    LLVMInitializeRISCVTarget, LLVMInitializeRISCVTargetInfo, LLVMInitializeRISCVTargetMC,
};

use std::ffi::CStr;

use std::fs::File;
use std::io::Write;
use std::ptr;

use crate::traptype::TrapType;

pub struct Itrace {
    disasm: *mut LLVMOpaqueDisasmContext,
    log_file: File,
}
unsafe impl Send for Itrace {}

impl Itrace {
    pub fn new(hart_id:usize) -> Self {
        let disasm: *mut LLVMOpaqueDisasmContext = unsafe {
            LLVMInitializeRISCVTargetInfo();
            LLVMInitializeRISCVTarget();
            LLVMInitializeRISCVTargetMC();
            LLVMInitializeRISCVAsmPrinter();
            LLVMInitializeRISCVAsmParser();
            LLVMInitializeRISCVDisassembler();
            // cstr is end of \0
            LLVMCreateDisasmCPUFeatures(
                "riscv64-pc-linux-gnu\0".as_ptr() as *const i8,
                "sifive-s76\0".as_ptr() as *const i8,
                ptr::null_mut(), // "+m,+a,+f,+d\0"
                ptr::null_mut(),
                0,
                None,
                None,
            )
        };

        if disasm.is_null() {
            panic!("Itrace init fail");
        };

        let path = format!("/tmp/rv64emu_itrace_logs_{}",hart_id);
        let fd = File::create(path).unwrap();

        Itrace {
            disasm,
            log_file: fd,
        }
    }

    pub fn disassemble_bytes(&mut self, pc: u64, inst: u32) {
        let mut sbuf = [0i8; 64];
        let mut x = inst.to_le_bytes();
        let _sz = unsafe {
            LLVMDisasmInstruction(
                self.disasm,
                x.as_mut_ptr(),
                x.len() as u64,
                pc,
                sbuf.as_mut_ptr() as *mut i8,
                sbuf.len(),
            )
        };
        let instr_str = unsafe { CStr::from_ptr(sbuf.as_ptr()) };

        let disasm_ret = format!("{:08x} {:08x} {}\n", pc, inst, instr_str.to_string_lossy());
        self.itrace_log(&disasm_ret);
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
