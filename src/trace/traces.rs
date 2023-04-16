use crate::traptype::TrapType;

#[cfg(feature = "rv_debug_trace")]
use super::{ftrace::Ftrace, itrace::Itrace};

pub enum TraceType {
    Itrace(u64, u32),         // (pc, inst)
    Call(u64, u64),           // (inst_pc,jump_pc)
    Return(u64, u64),         // (inst_pc,jump_pc)
    Trap(TrapType, u64, u64), //trap_type: TrapType, epc: u64, tval: u64
}

pub struct Traces {
    #[cfg(feature = "rv_debug_trace")]
    pub itrace: Itrace,
    #[cfg(feature = "rv_debug_trace")]
    pub ftrace: Ftrace,
    receiver: crossbeam_channel::Receiver<TraceType>,
}

impl Traces {
    pub fn new(hart_id:usize,receiver: crossbeam_channel::Receiver<TraceType>) -> Self {

        Traces {
            #[cfg(feature = "rv_debug_trace")]
            itrace: Itrace::new(hart_id),
            #[cfg(feature = "rv_debug_trace")]
            ftrace: Ftrace::new(hart_id),
            receiver,
        }
    }

    pub fn run(&mut self) {
        #[cfg(feature = "rv_debug_trace")]
        while !self.receiver.is_empty() {
            match self.receiver.recv() {
                Ok(TraceType::Itrace(pc, inst)) => {
                    self.itrace.disassemble_bytes(pc, inst);
                }
                Ok(TraceType::Trap(trap_type, epc, tval)) => {
                    self.itrace.trap_record(trap_type, epc, tval);
                }
                Ok(TraceType::Call(inst_pc, pc)) => {
                    self.ftrace.call_record(inst_pc, pc);
                }
                Ok(TraceType::Return(inst_pc, pc)) => {
                    self.ftrace.ret_record(inst_pc, pc);
                }
                Err(_) => {}
            }
        }
    }
}
