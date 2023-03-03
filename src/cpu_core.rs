use std::{fs::File, io::Write};

use crate::{
    bus::Bus,
    clint::{Clint, DeviceClint},
    cpu_icache::CpuIcache,
    csr_regs::CsrRegs,
    csr_regs_define::{Mip, Mstatus, Mtvec, Stvec},
    gpr::Gpr,
    inst_base::{
        PrivilegeLevels, CSR_MCAUSE, CSR_MEDELEG, CSR_MEPC, CSR_MIDELEG, CSR_MIE, CSR_MIP,
        CSR_MSTATUS, CSR_MTVEC, CSR_SCAUSE, CSR_SEPC, CSR_STVEC, MASK_ALL, MIP_MTIP,
    },
    inst_decode::InstDecode,
    inst_rv64a::LrScReservation,
    traptype::TrapType, itrace::Itrace,
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
    pub cur_priv: PrivilegeLevels,
    pub lr_sc_set: LrScReservation, // for rv64a inst
    pub cpu_state: CpuState,
    pub cpu_icache: CpuIcache,
    pub itrace:Itrace,
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
                self.itrace.disassemble_bytes(self.pc, inst);
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
                    self.step(inst);

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
        let medeleg_val = self.csr_regs.read_raw_mask(CSR_MEDELEG.into(), MASK_ALL);
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

            self.csr_regs
                .write_raw_mask(CSR_MSTATUS.into(), mstatus.into(), MASK_ALL);

            self.csr_regs
                .write_raw_mask(CSR_SEPC.into(), self.pc, MASK_ALL);

            self.csr_regs
                .write_raw_mask(CSR_SCAUSE.into(), cause, MASK_ALL);

            let stvec_val = self.csr_regs.read_raw_mask(CSR_STVEC.into(), MASK_ALL);
            self.npc = Stvec::from(stvec_val).get_trap_pc(trap_type);
            self.cur_priv = PrivilegeLevels::Supervisor;
        }
        // exception to M mode
        else {
            mstatus.set_mpie(mstatus.mie());
            mstatus.set_mie(false);
            mstatus.set_mpp(self.cur_priv as u8);
            self.csr_regs
                .write_raw_mask(CSR_MSTATUS.into(), mstatus.into(), MASK_ALL);
            self.csr_regs
                .write_raw_mask(CSR_MEPC.into(), self.pc, MASK_ALL);
            self.csr_regs
                .write_raw_mask(CSR_MCAUSE.into(), trap_type as u64, MASK_ALL);

            let mtvec_val = self.csr_regs.read_raw_mask(CSR_MTVEC.into(), MASK_ALL);
            self.npc = Mtvec::from(mtvec_val).get_trap_pc(trap_type);
            self.cur_priv = PrivilegeLevels::Machine;
        }


    }

    pub fn handle_interrupt(&mut self) {
        // read necessary csrs

        let mip_val = self.csr_regs.read_raw_mask(CSR_MIP.into(), MASK_ALL);
        let mie_val = self.csr_regs.read_raw_mask(CSR_MIE.into(), MASK_ALL);

        let mip_mie_val = mip_val & mie_val;
        // no interupt allowed
        if mip_mie_val == 0 {
            return;
        }

        let mstatus_val = self.csr_regs.read_raw_mask(CSR_MSTATUS.into(), MASK_ALL);
        let mut mstatus = Mstatus::from(mstatus_val);
        let mideleg_val = self.csr_regs.read_raw_mask(CSR_MIDELEG.into(), MASK_ALL);

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
            self.csr_regs
                .write_raw_mask(CSR_MSTATUS.into(), mstatus.into(), MASK_ALL);
            self.csr_regs
                .write_raw_mask(CSR_MEPC.into(), self.npc, MASK_ALL);
            self.csr_regs
                .write_raw_mask(CSR_MCAUSE.into(), cause as u64, MASK_ALL);
            let mtvec_val = self.csr_regs.read_raw_mask(CSR_MTVEC.into(), MASK_ALL);
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

            self.csr_regs
                .write_raw_mask(CSR_MSTATUS.into(), mstatus.into(), MASK_ALL);

            self.csr_regs
                .write_raw_mask(CSR_SEPC.into(), self.npc, MASK_ALL);

            self.csr_regs
                .write_raw_mask(CSR_SCAUSE.into(), cause as u64, MASK_ALL);

            let stvec_val = self.csr_regs.read_raw_mask(CSR_STVEC.into(), MASK_ALL);
            self.npc = Stvec::from(stvec_val).get_trap_pc(cause);
            self.cur_priv = PrivilegeLevels::Supervisor;
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
