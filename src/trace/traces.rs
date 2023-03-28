use crate::traptype::TrapType;

use super::{ftrace::Ftrace, itrace::Itrace};

pub enum TraceType {
    Itrace(u64, u32),         // (pc, inst)
    Call(u64, u64),           // (inst_pc,jump_pc)
    Return(u64, u64),         // (inst_pc,jump_pc)
    Trap(TrapType, u64, u64), //trap_type: TrapType, epc: u64, tval: u64
}

pub struct Traces {
    pub itrace: Itrace,
    pub ftrace: Ftrace,
    receiver: crossbeam_channel::Receiver<TraceType>,
}

impl Traces {
    pub fn new(receiver: crossbeam_channel::Receiver<TraceType>) -> Self {
        Traces {
            itrace: Itrace::new(),
            ftrace: Ftrace::new(),
            receiver,
        }
    }

    pub fn run(&mut self) {
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
