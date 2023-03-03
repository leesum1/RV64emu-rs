use llvm_sys::disassembler::{LLVMCreateDisasm, LLVMDisasmInstruction, LLVMOpaqueDisasmContext};
use llvm_sys::target::{
    LLVM_InitializeAllAsmParsers, LLVM_InitializeAllDisassemblers, LLVM_InitializeAllTargetInfos,
    LLVM_InitializeAllTargetMCs,
};
use std::ffi::CStr;

use std::ptr;

pub struct Itrace {
    disasm: *mut LLVMOpaqueDisasmContext,
}
unsafe impl Send for Itrace {}

impl Itrace {
    pub fn new() -> Self {
        let disasm: *mut LLVMOpaqueDisasmContext = unsafe {
            LLVM_InitializeAllTargetInfos();
            LLVM_InitializeAllTargetMCs();
            LLVM_InitializeAllDisassemblers();
            LLVM_InitializeAllAsmParsers();
            LLVMCreateDisasm(
                "riscv64-pc-linux-gnu\0".as_ptr() as *const i8,
                ptr::null_mut(),
                0,
                None,
                None,
            )
        };
        if disasm.is_null() {
            panic!("Itrace init fail");
        };
        Itrace { disasm }
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
        println!("{:08x} {:08x} {}", pc, inst, instr_str.to_string_lossy());
    }
}
