use crate::{
    bus::{Bus, DeviceType},
    dram::Dram,
    gpr::Gpr,
    inst_decode::{self, InstDecode},
    traptype::TrapType,
};

#[derive(PartialEq)]
pub enum CpuState {
    Running,
    Stop,
    Abort,
}

pub struct CpuCore {
    pub gpr: Gpr,
    pub bus: Bus,
    pub decode: InstDecode,
    pub pc: u64,
    pub npc: u64,
    pub cpu_state: CpuState,
}

impl CpuCore {
    pub fn new() -> Self {
        let mut bus_u = Bus::new();

        CpuCore {
            gpr: Gpr::new(),
            decode: InstDecode::new(),
            bus: bus_u,
            pc: 0x8000_0000,
            npc: 0x8000_0000,
            cpu_state: CpuState::Stop,
        }
    }

    pub fn inst_fetch(&mut self) -> u32 {
        self.pc = self.npc;
        self.npc += 4;
        let inst = self.bus.read(self.pc, 4);
        inst as u32
    }

    pub fn step(&mut self, inst: u32) {
        let x = self.decode.step(inst);

        match x {
            Some(i) => {
                println!("{}", i.name);
                (i.operation)(self, inst, self.pc);
            }
            None => panic!(),
        }
    }

    pub fn execute(&mut self, num: usize) {
        for _ in 0..num {
            match self.cpu_state {
                CpuState::Running => {
                    let inst = self.inst_fetch();
                    self.step(inst);
                }
                CpuState::Stop => break,
                CpuState::Abort => break,
            };
        }
    }

    pub fn halt(&mut self) -> usize {
        let a0 = self.gpr.read_by_name("a0");
        match a0 {
            0 => {
                println!("GOOD TRAP");
                self.cpu_state = CpuState::Stop;
                0
            }
            _ => {
                println!("BAD TRAP");
                1
            }
        }
    }
}

#[cfg(test)]
mod tests_cpu {
    use crate::{bus::DeviceType, dram::Dram};

    use super::{CpuCore, CpuState};

    #[test]
    fn test1() {
        let file_name =
            "/home/leesum/workhome/ysyx/am-kernels/tests/cpu-tests/build/mul-longlong-riscv64-nemu.bin";
        let mut cpu = CpuCore::new();
        let mut dr = Box::new(Dram::new(128 * 1024 * 1024));
        dr.load_binary(file_name);

        let dram_u = DeviceType {
            start: 0x8000_0000,
            len: dr.capacity as u64,
            instance: dr,
        };

        cpu.bus.add_device(dram_u);

        cpu.cpu_state = CpuState::Running;

        let mut cycle = 0;
        loop {
            cpu.execute(1);
            cycle +=1;
            if cpu.cpu_state != CpuState::Running {
                break;
            }
        }

        println!("total:{cycle}");
    }
}


