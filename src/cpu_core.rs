use std::{fs::File, io::Write};

use crate::{
    bus::Bus,
    sifive_clint::{Clint, DeviceClint},
    cpu_icache::CpuIcache,
    csr_regs::CsrRegs,
    csr_regs_define::{Mip, Mstatus, Mtvec, Stvec},
    gpr::Gpr,
    inst::inst_base::{
        PrivilegeLevels, CSR_MCAUSE, CSR_MEDELEG, CSR_MEPC, CSR_MIDELEG, CSR_MIE, CSR_MIP,
        CSR_MSTATUS, CSR_MTVEC, CSR_SCAUSE, CSR_SEPC, CSR_STVEC,
    },
    inst_decode::InstDecode,
    inst::inst_rv64a::LrScReservation,
    itrace::Itrace,
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
    pub csr_regs: CsrRegs,
    pub bus: Bus,
    pub decode: InstDecode,
    pub pc: u64,
    pub npc: u64,
    pub cur_priv: PrivilegeLevels,
    pub lr_sc_set: LrScReservation, // for rv64a inst
    pub cpu_state: CpuState,
    pub cpu_icache: CpuIcache,
    pub itrace: Itrace,
    pub debug_flag: bool,
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
            cur_priv: PrivilegeLevels::Machine,
            itrace: Itrace::new(),
            debug_flag: true,
        }
    }

    pub fn inst_fetch(&mut self) -> Result<u32, TrapType> {
        self.pc = self.npc;
        self.npc += 4;

        // first lookup icache
        let icache_data = self.cpu_icache.get_inst(self.pc);
        // if icache hit return, else load inst from mem and push into icache
        match icache_data {
            Some(icache_inst) => Ok(icache_inst),
            None => {
                let inst_val = self.bus.read(self.pc, 4).unwrap();
                self.cpu_icache.insert_inst(self.pc, inst_val as u32);
                Ok(inst_val as u32)
            }
        }

        // self.bus.read(self.pc, 4) as u32
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
                if self.debug_flag {
                    self.itrace.disassemble_bytes(self.pc, inst);
                }
                let trap_code = (i.operation)(self, inst, self.pc);
                if let Err(e) = trap_code {
                    self.handle_exceptions(e)
                }
            }
            None => panic!("inst err,pc:{:X},inst:{:x}", self.pc, inst),
        }
    }

    pub fn execute(&mut self, num: usize) {
        for _ in 0..num {
            match self.cpu_state {
                CpuState::Running => {
                    let inst = self.inst_fetch();

                    match inst {
                        Ok(inst_val) => self.step(inst_val),
                        Err(trap_type) => {
                            self.handle_exceptions(trap_type);
                            continue;
                        }
                    };

                    self.check_pending_int();
                    self.handle_interrupt();
                    self.bus.update();
                }
                _ => break,
            };
        }
    }
    fn check_pending_int(&mut self) {
        let mip_val = self.csr_regs.read_raw(CSR_MIP.into());
        let mut mip = Mip::from(mip_val);

        let irq_clint = self.bus.clint.instance.is_interrupt();
        mip.set_mtip(irq_clint);
        self.csr_regs.write_raw(CSR_MIP.into(), mip.into());
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
        let mstatus_val = self.csr_regs.read_raw(CSR_MSTATUS.into());
        let medeleg_val = self.csr_regs.read_raw(CSR_MEDELEG.into());
        let has_exception = (medeleg_val & (1_u64 << trap_type.get_exception_num())) != 0;
        let trap_to_s_enable = self.cur_priv <= PrivilegeLevels::Supervisor;
        let mut mstatus = Mstatus::from(mstatus_val);

        // exception to S mode
        if has_exception & trap_to_s_enable {
            let cause = trap_type as u64;
            // When a trap is taken, SPP is set to 0 if the trap originated from user mode, or 1 otherwise.
            mstatus.set_spp(!(self.cur_priv == PrivilegeLevels::User));
            // When a trap is taken into supervisor mode, SPIE is set to SIE
            mstatus.set_spie(mstatus.sie());
            // and SIE is set to 0
            mstatus.set_sie(false);

            self.csr_regs.write_raw(CSR_MSTATUS.into(), mstatus.into());

            self.csr_regs.write_raw(CSR_SEPC.into(), self.pc);

            self.csr_regs.write_raw(CSR_SCAUSE.into(), cause);

            let stvec_val = self.csr_regs.read_raw(CSR_STVEC.into());
            self.npc = Stvec::from(stvec_val).get_trap_pc(trap_type);
            self.cur_priv = PrivilegeLevels::Supervisor;
        }
        // exception to M mode
        else {
            mstatus.set_mpie(mstatus.mie());
            mstatus.set_mie(false);
            mstatus.set_mpp(self.cur_priv as u8);
            self.csr_regs.write_raw(CSR_MSTATUS.into(), mstatus.into());
            self.csr_regs.write_raw(CSR_MEPC.into(), self.pc);
            self.csr_regs.write_raw(CSR_MCAUSE.into(), trap_type as u64);

            let mtvec_val = self.csr_regs.read_raw(CSR_MTVEC.into());
            self.npc = Mtvec::from(mtvec_val).get_trap_pc(trap_type);
            self.cur_priv = PrivilegeLevels::Machine;
        }
    }

    pub fn handle_interrupt(&mut self) {
        // read necessary csrs

        let mip_val = self.csr_regs.read_raw(CSR_MIP.into());
        let mie_val = self.csr_regs.read_raw(CSR_MIE.into());

        let mip_mie_val = mip_val & mie_val;
        // no interupt allowed
        if mip_mie_val == 0 {
            return;
        }

        let mstatus_val = self.csr_regs.read_raw(CSR_MSTATUS.into());
        let mut mstatus = Mstatus::from(mstatus_val);
        let mideleg_val = self.csr_regs.read_raw(CSR_MIDELEG.into());

        // let _mideleg = Mip::from(mideleg_val);
        // let _mip_mie = MieMip::from(mip_mie_val);
        // println!("{_mideleg:?}");
        // println!("{_mip_mie:?},{:b}",u64::from(_mip_mie));

        let m_a1 = mstatus.mie() & (self.cur_priv == PrivilegeLevels::Machine);
        let m_a2 = self.cur_priv < PrivilegeLevels::Machine;
        let int_to_m_enable = m_a1 | m_a2;
        let int_to_m_peding = mip_mie_val & !mideleg_val;

        let s_a1 = mstatus.sie() & (self.cur_priv == PrivilegeLevels::Supervisor);
        let s_a2 = self.cur_priv < PrivilegeLevels::Supervisor;
        let int_to_s_enable = s_a1 | s_a2;
        let int_to_s_peding = mip_mie_val & mideleg_val;

        // handing interupt in M mode
        if int_to_m_enable && int_to_m_peding != 0 {
            let cause = Mip::from(int_to_m_peding).get_priority_interupt();
            mstatus.set_mpie(mstatus.mie());
            mstatus.set_mpp(self.cur_priv as u8);
            mstatus.set_mie(false);
            self.csr_regs.write_raw(CSR_MSTATUS.into(), mstatus.into());
            self.csr_regs.write_raw(CSR_MEPC.into(), self.npc);
            self.csr_regs.write_raw(CSR_MCAUSE.into(), cause as u64);
            let mtvec_val = self.csr_regs.read_raw(CSR_MTVEC.into());
            // todo! improve me
            self.npc = Mtvec::from(mtvec_val).get_trap_pc(cause);
            self.cur_priv = PrivilegeLevels::Machine;
        }
        // handing interupt in S mode
        // The sstatus register is a subset of the mstatus register.
        // In a straightforward implementation, reading or writing any field in sstatus is equivalent to
        // reading or writing the homonymous field in mstatus.
        // so, we use mstatus instead of sstatus below
        else if int_to_s_enable && int_to_s_peding != 0 {
            let cause = Mip::from(int_to_s_peding).get_priority_interupt();
            // When a trap is taken, SPP is set to 0 if the trap originated from user mode, or 1 otherwise.
            mstatus.set_spp(!(self.cur_priv == PrivilegeLevels::User));
            // When a trap is taken into supervisor mode, SPIE is set to SIE
            mstatus.set_spie(mstatus.sie());
            // and SIE is set to 0
            mstatus.set_sie(false);

            self.csr_regs.write_raw(CSR_MSTATUS.into(), mstatus.into());

            self.csr_regs.write_raw(CSR_SEPC.into(), self.npc);

            self.csr_regs.write_raw(CSR_SCAUSE.into(), cause as u64);

            let stvec_val = self.csr_regs.read_raw(CSR_STVEC.into());
            self.npc = Stvec::from(stvec_val).get_trap_pc(cause);
            self.cur_priv = PrivilegeLevels::Supervisor;
        }
    }
    // for riscof
    pub fn dump_signature(&mut self, file_name: &str) {
        let fd = File::create(file_name);

        let sig_start = self.gpr.read_by_name("a1");
        let sig_end = self.gpr.read_by_name("a2");

        fd.map_or_else(
            |err| println!("{err}"),
            |mut file| {
                for i in (sig_start..sig_end).step_by(4) {
                    let tmp_data = self.bus.read(i, 4).unwrap();
                    file.write_fmt(format_args! {"{tmp_data:08x}\n"}).unwrap();
                }
            },
        )
    }
    // for riscv-tests
    // It seems in riscv-tests ends with end code
    // written to a certain physical memory address
    // (0x80001000 in mose test cases) so checking
    // the data in the address and terminating the test
    // if non-zero data is written.
    // End code 1 seems to mean pass.
    pub fn check_to_host(&mut self) {
        let data = self.bus.read(0x8000_1000, 4).unwrap();
        match data {
            0 => (),
            1 => self.cpu_state = CpuState::Stop,
            2_u64..=u64::MAX => self.cpu_state = CpuState::Abort,
        };
    }
}

#[cfg(test)]
mod tests_cpu {
    use std::{fs::read_dir, path::Path};

    use crate::{bus::DeviceType, device::device_dram::DeviceDram};

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
            cpu.check_to_host();
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
