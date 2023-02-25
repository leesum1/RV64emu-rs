use std::{fs::File, io::Write};

use crate::{
    bus::Bus,
    clint::{Clint, DeviceClint},
    cpu_icache::CpuIcache,
    csr_regs::{CsrRW, CsrRegs},
    gpr::Gpr,
    inst_base::{
        get_field, set_field, CSR_MCAUSE, CSR_MEPC, CSR_MIE, CSR_MIP, CSR_MSTATUS, CSR_MTVEC,
        MASK_ALL, MIP_MTIP, MSTATUS_MIE, MSTATUS_MPIE, MSTATUS_MPP,
    },
    inst_decode::InstDecode,
    inst_rv64a::LrScReservation,
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
    pub lr_sc_set: LrScReservation, // for rv64a inst
    pub cpu_state: CpuState,
    pub cpu_icache: CpuIcache,
}

impl CpuCore {
    pub fn new() -> Self {
        // todo! improve me
        let clint_u = Clint::new();
        let device_clint = DeviceClint {
            start: 0x2000000,
            len: 0xBFFF,
            instance: clint_u,
            name: "Clint",
        };

        let bus_u = Bus::new(device_clint);

        CpuCore {
            gpr: (Gpr::new()),
            decode: InstDecode::new(),
            bus: bus_u,
            pc: 0x8000_0000,
            npc: 0x8000_0000,
            lr_sc_set: LrScReservation::new(),
            cpu_state: CpuState::Stop,
            csr_regs: CsrRegs::new(),
            cpu_icache: CpuIcache::new(),
        }
    }

    pub fn inst_fetch(&mut self) -> u32 {
        self.pc = self.npc;
        self.npc += 4;

        // first lookup icache
        let icache_data = self.cpu_icache.get_inst(self.pc);
        // if icache hit return, else load inst from mem and push into icache
        match icache_data {
            Some(icache_inst) => icache_inst,
            None => {
                let inst = self.bus.read(self.pc, 4) as u32;
                self.cpu_icache.insert_inst(self.pc, inst);
                inst
            }
        }
    }

    pub fn step(&mut self, inst: u32) {
        let fast_decode = self.decode.fast_path(inst);

        let inst_op = if fast_decode.is_some() {
            fast_decode
        } else {
            self.decode.slow_path(inst)
        };

        // let inst_op = self.decode.slow_path(inst);

        match inst_op {
            Some(i) => {
                // println!("{:x},{}",self.pc,i.name);
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

                    // let mip_p: &mut Box<dyn CsrRW> =
                    //     self.csr_regs.csr_map.get_mut(&(CSR_MIP as u64)).unwrap();

                    let irq_clint = self.bus.clint.instance.is_interrupt();
                    // println!("irq_clint:{irq_clint}");

                    self.csr_regs
                        .write_raw_mask(CSR_MIP.into(), irq_clint as u64, MIP_MTIP);

                    // // println!("mip:{:x}",x);
                    self.handle_interrupt();
                    self.bus.update();
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
        let mstatus_val = self.csr_regs.read_raw_mask(CSR_MSTATUS.into(), MASK_ALL);

        let mstatus_mie_bit = get_field(mstatus_val, MSTATUS_MIE);
        let mstatus_val = set_field(mstatus_val, MSTATUS_MPIE, mstatus_mie_bit);
        let mstatus_val = set_field(mstatus_val, MSTATUS_MPP, 0b11);
        let mstatus_val = set_field(mstatus_val, MSTATUS_MIE, 0b0);

        self.csr_regs
            .write_raw_mask(CSR_MSTATUS.into(), mstatus_val, MASK_ALL);
        self.csr_regs
            .write_raw_mask(CSR_MEPC.into(), self.pc, MASK_ALL);
        self.csr_regs
            .write_raw_mask(CSR_MCAUSE.into(), trap_type as u64, MASK_ALL);

        let mtvec = self.csr_regs.read_raw_mask(CSR_MTVEC.into(), MASK_ALL);

        self.npc = mtvec;
    }

    pub fn handle_interrupt(&mut self) {
        let mstatus_val = self.csr_regs.read_raw_mask(CSR_MSTATUS.into(), MASK_ALL);
        // println!("mstatus:{:x}", mstatus_val);
        let mstatus_mie = get_field(mstatus_val, MSTATUS_MIE) == 1;

        let mip_val = self.csr_regs.read_raw_mask(CSR_MIP.into(), MASK_ALL);
        let mie_val = self.csr_regs.read_raw_mask(CSR_MIE.into(), MASK_ALL);
        let mie_and_mip = mip_val & mie_val;

        let machine_timer_interrupt = get_field(mie_and_mip, MIP_MTIP) == 1;

        if machine_timer_interrupt & mstatus_mie {
            // println!("IRQ:mstatus_pre:{mstatus_val:x}");

            let mstatus_val = set_field(mstatus_val, MSTATUS_MPIE, mstatus_mie as u64);
            let mstatus_val = set_field(mstatus_val, MSTATUS_MPP, 0b11);
            let mstatus_val = set_field(mstatus_val, MSTATUS_MIE, 0b0);

            // println!("IRQ:mstatus_now:{mstatus_val:x}");
            self.csr_regs
                .write_raw_mask(CSR_MSTATUS.into(), mstatus_val, MASK_ALL);
            self.csr_regs
                .write_raw_mask(CSR_MEPC.into(), self.npc, MASK_ALL);
            self.csr_regs.write_raw_mask(
                CSR_MCAUSE.into(),
                TrapType::MachineTimerInterrupt as u64,
                MASK_ALL,
            );
            let mtvec_val = self.csr_regs.read_raw_mask(CSR_MTVEC.into(), MASK_ALL);
            self.npc = mtvec_val;
        }
    }

    pub fn dump_signature(&mut self, file_name: &str) {
        let fd = File::create(file_name);

        let sig_start = self.gpr.read_by_name("a1");
        let sig_end = self.gpr.read_by_name("a2");

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
