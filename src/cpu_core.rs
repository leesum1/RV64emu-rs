use std::{fs::File, io::Write};

use crate::{
    bus::Bus,
    csr_regs::CsrRegs,
    gpr::Gpr,
    inst_base::{
        get_field, set_field, CSR_MCAUSE, CSR_MEPC, CSR_MSTATUS, CSR_MTVEC, MSTATUS_MIE,
        MSTATUS_MPIE, MSTATUS_MPP,
    },
    inst_decode::InstDecode,
    traptype::TrapType,
};

#[derive(PartialEq)]
pub enum CpuState {
    Running,
    Stop,
    // Abort,
}

pub struct CpuCore {
    pub gpr: Gpr,
    pub csr_regs: CsrRegs,
    pub bus: Bus,
    pub decode: InstDecode,
    pub pc: u64,
    pub npc: u64,
    pub cpu_state: CpuState,
}

impl CpuCore {
    pub fn new() -> Self {
        let bus_u = Bus::new();

        CpuCore {
            gpr: Gpr::new(),
            decode: InstDecode::new(),
            bus: bus_u,
            pc: 0x8000_0000,
            npc: 0x8000_0000,
            cpu_state: CpuState::Stop,
            csr_regs: CsrRegs::new(),
        }
    }

    pub fn inst_fetch(&mut self) -> u32 {
        self.pc = self.npc;
        self.npc += 4;
        let inst = self.bus.read(self.pc, 4);
        inst as u32
    }

    pub fn step(&mut self, inst: u32) {
        let y = self.decode.step(inst);
        match y {
            Some(i) => {
                let trap_code = (i.operation)(self, inst, self.pc);

                if let Err(e) = trap_code {
                    self.handle_exceptions(e)
                }
            }
            None => panic!("inst err:{inst:X}"),
        }
    }

    pub fn execute(&mut self, num: usize) {
        for _ in 0..num {
            match self.cpu_state {
                CpuState::Running => {
                    let inst = self.inst_fetch();
                    self.step(inst);
                }
                _ => break,
            };
        }
    }

    pub fn halt(&mut self) -> usize {
        let a0 = self.gpr.read_by_name("a0");

        if let 0 = a0 {
            println!("GOOD TRAP");
        } else {
            println!("BAD TRAP");
        }
        self.cpu_state = CpuState::Stop;
        a0 as usize
    }

    pub fn handle_exceptions(&mut self, trap_type: TrapType) {
        let mstatus_val = self.csr_regs.read(CSR_MSTATUS.into());

        let mie = get_field(mstatus_val, MSTATUS_MIE);
        let mstatus_val = set_field(mstatus_val, MSTATUS_MPIE, mie);
        let mstatus_val = set_field(mstatus_val, MSTATUS_MPP, 0b11);
        let mstatus_val = set_field(mstatus_val, MSTATUS_MIE, 0b0);

        self.csr_regs.write(CSR_MSTATUS.into(), mstatus_val);
        self.csr_regs.write(CSR_MEPC.into(), self.pc);
        self.csr_regs.write(CSR_MCAUSE.into(), trap_type as u64);

        let mtvec = self.csr_regs.read(CSR_MTVEC.into());

        self.npc = mtvec;
    }

    pub fn dump_signature(&mut self, file_name: &str) {
        let fd = File::create(file_name);

        let sig_start = self.gpr.read_by_name("a1");
        let sig_end = self.gpr.read_by_name("a2");

        // for i in (sig_start..sig_end).step_by(4) {
        //     let tmp_data = self.bus.read(i, 4);
        //     file.write_fmt(format_args! {"{tmp_data:x}\n"});
        // }

        fd.map_or_else(
            |err| println!("{err}"),
            |mut file| {
                for i in (sig_start..sig_end).step_by(4) {
                    let tmp_data = self.bus.read(i, 4);
                    file.write_fmt(format_args! {"{tmp_data:08x}\n"}).unwrap();
                }
            },
        )
    }
}

#[cfg(test)]
mod tests_cpu {
    use std::{fs::read_dir, path::Path};

    use crate::{bus::DeviceType, device_dram::DeviceDram};

    use super::{CpuCore, CpuState};

    fn run_once(file_name: &str) {
        // let file_name =
        //     "/home/leesum/workhome/ysyx/am-kernels/tests/cpu-tests/build/mul-longlong-riscv64-nemu.bin";
        let mut cpu = CpuCore::new();
        let mut dr = Box::new(DeviceDram::new(128 * 1024 * 1024));
        dr.load_binary(file_name);

        let dram_u = DeviceType {
            start: 0x8000_0000,
            len: dr.capacity as u64,
            instance: dr,
            name: "DRAM",
        };

        cpu.bus.add_device(dram_u);

        cpu.cpu_state = CpuState::Running;

        let mut cycle = 0;
        loop {
            cpu.execute(1);
            cycle += 1;
            if cpu.cpu_state != CpuState::Running {
                break;
            }
        }
        println!("total:{cycle}");
        let a0_val = cpu.gpr.read_by_name("a0");
        assert_eq!(a0_val, 0);
    }

    #[test]
    fn cpu_test() {
        let dir = Path::new("/home/leesum/workhome/ysyx/am-kernels/tests/cpu-tests/build");
        for file in read_dir(dir).unwrap() {
            let entry = file.unwrap();
            let path = entry.path();

            if path.is_file() {
                let ext = path.extension().unwrap();
                if ext == "bin" {
                    run_once(path.to_str().unwrap());
                    let f_name = path.file_name().unwrap().to_str().unwrap();
                    println!("{f_name}:  OK");
                }
            }
        }
    }
    #[test]
    fn test1() {
        run_once("/home/leesum/workhome/ysyx/am-kernels/tests/cpu-tests/build/recursion-riscv64-nemu.bin");
    }
}
