use std::{cell::Cell, fs::File, io::Write, rc::Rc};

use crate::{
    cpu_icache::CpuIcache,
    csr_regs::CsrRegs,
    csr_regs_define::XipIn,
    gpr::Gpr,
    inst::inst_base::PrivilegeLevels,
    inst::{
        inst_base::{AccessType, FesvrCmd},
        inst_rv64a::LrScReservation,
    },
    inst_decode::InstDecode,
    mmu::Mmu,
    trace::traces::TraceType,
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
    pub mmu: Mmu,
    pub decode: InstDecode,
    pub pc: u64,
    pub npc: u64,
    pub cur_priv: Rc<Cell<PrivilegeLevels>>,
    pub lr_sc_set: LrScReservation, // for rv64a inst
    pub cpu_state: CpuState,
    pub cpu_icache: CpuIcache,
    pub trace_sender: Option<crossbeam_channel::Sender<TraceType>>,
}
unsafe impl Send for CpuCore {}
impl CpuCore {
    pub fn new(trace_sender: Option<crossbeam_channel::Sender<TraceType>>) -> Self {
        // todo! improve me
        let csr_regs_u = CsrRegs::new();
        let privi_u = Rc::new(Cell::new(PrivilegeLevels::Machine));
        let mstatus = csr_regs_u.xstatus.clone();
        let satp = csr_regs_u.satp.clone();
        let mtime = csr_regs_u.time.clone();

        let mmu_u = Mmu::new(privi_u.clone(), mstatus, satp, mtime);

        CpuCore {
            gpr: (Gpr::new()),
            decode: InstDecode::new(),
            mmu: mmu_u,
            pc: 0x8000_0000,
            npc: 0x8000_0000,
            lr_sc_set: LrScReservation::new(),
            cpu_state: CpuState::Stop,
            csr_regs: csr_regs_u,
            cpu_icache: CpuIcache::new(),
            cur_priv: privi_u,
            trace_sender,
        }
    }

    pub fn inst_fetch(&mut self) -> Result<u64, TrapType> {
        self.pc = self.npc;
        self.npc += 4;

        // // first lookup icache
        // let icache_data = self.cpu_icache.get_inst(self.pc);
        // // if icache hit return, else load inst from mem and push into icache
        // let fetch_ret = match icache_data {
        //     Some(icache_inst) => Ok(icache_inst as u64),
        //     None => self.read(self.pc, 4, AccessType::Fetch(self.pc)),
        // };

        // if let Ok(inst) = fetch_ret {
        //     self.cpu_icache.insert_inst(self.pc, inst as u32)
        // }
        // fetch_ret
        self.read(self.pc, 4, AccessType::Fetch(self.pc))
    }

    pub fn step(&mut self, inst: u32) -> Result<(), TrapType> {
        // let fast_decode = self.decode.fast_path(inst);
        // let inst_op = if fast_decode.is_some() {
        //     fast_decode
        // } else {
        //     self.decode.slow_path(inst)
        // };

        let inst_op = self.decode.slow_path(inst);

        match inst_op {
            Some(i) => {
                #[cfg(feature = "rv_debug_trace")]
                if let Some(sender) = &self.trace_sender {
                    sender.send(TraceType::Itrace(self.pc, inst)).unwrap();
                };
                (i.operation)(self, inst, self.pc) // return
            }
            None => {
                println!("inst err,pc:{:X},inst:{:x}", self.pc, inst);
                Err(TrapType::IllegalInstruction(inst.into()))
            }
        }
    }

    pub fn execute(&mut self, num: usize) {
        for _ in 0..num {
            match self.cpu_state {
                CpuState::Running => {
                    let fetch_ret = self.inst_fetch();

                    let exe_ret = match fetch_ret {
                        // op ret
                        Ok(inst_val) => self.step(inst_val as u32),
                        // fetch fault
                        Err(trap_type) => Err(trap_type),
                    };

                    if let Err(trap_type) = exe_ret {
                        self.handle_exceptions(trap_type);
                        continue;
                    }

                    self.check_pending_int();
                    self.handle_interrupt();
                    // Increment the cycle counter
                    let cycle = self.csr_regs.cycle.get();
                    self.csr_regs.cycle.set(cycle + 1);
                    // Increment the instruction counter
                    let instret = self.csr_regs.instret.get();
                    self.csr_regs.instret.set(instret + 1);

                    self.mmu.bus.update();
                }
                _ => break,
            };
        }
    }
    fn check_pending_int(&mut self) {
        let clint = &self.mmu.bus.clint.instance;
        let mut mip = self.csr_regs.xip.get();
        let irq_clint = clint.is_interrupt();
        mip.set_mtip(irq_clint);
        self.csr_regs.xip.set(mip);
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
        // let mstatus_val = self.csr_regs.read_raw(CSR_MSTATUS.into());
        // let medeleg_val = self.csr_regs.read_raw(CSR_MEDELEG.into());
        let medeleg = self.csr_regs.medeleg.get();
        let mut mstatus = self.csr_regs.xstatus.get();

        // let has_exception = (medeleg_val & (1_u64 << trap_type.get_exception_num())) != 0;
        let has_exception = u64::from(medeleg) & (1_u64 << trap_type.get_exception_num()) != 0;

        let trap_to_s_enable = self.cur_priv.get() <= PrivilegeLevels::Supervisor;
        // let mut mstatus = Mstatus::from(mstatus_val);

        let tval = trap_type.get_tval();
        let cause = trap_type.idx();

        // exception to S mode
        if has_exception & trap_to_s_enable {
            // When a trap is taken, SPP is set to 0 if the trap originated from user mode, or 1 otherwise.
            mstatus.set_spp(!(self.cur_priv.get() == PrivilegeLevels::User));
            // When a trap is taken into supervisor mode, SPIE is set to SIE
            mstatus.set_spie(mstatus.sie());
            // and SIE is set to 0
            mstatus.set_sie(false);

            // self.csr_regs.write_raw(CSR_MSTATUS.into(), mstatus.into());

            // self.csr_regs.write_raw(CSR_SEPC.into(), self.pc);

            // self.csr_regs.write_raw(CSR_SCAUSE.into(), cause);

            // self.csr_regs.write_raw(CSR_STVAL.into(), tval);

            self.csr_regs.xstatus.set(mstatus);
            self.csr_regs.sepc.set(self.pc);
            self.csr_regs.scause.set(cause.into());
            self.csr_regs.stval.set(tval);

            if let Some(sender) = &self.trace_sender {
                sender
                    .send(TraceType::Trap(trap_type, self.pc, tval))
                    .unwrap();
            };

            // let stvec_val = self.csr_regs.read_raw(CSR_STVEC.into());
            let stvec = self.csr_regs.stvec.get();

            self.npc = stvec.get_trap_pc(trap_type);
            self.cur_priv.set(PrivilegeLevels::Supervisor);
        }
        // exception to M mode
        else {
            mstatus.set_mpie(mstatus.mie());
            mstatus.set_mie(false);
            mstatus.set_mpp(self.cur_priv.get() as u8);
            // self.csr_regs.write_raw(CSR_MSTATUS.into(), mstatus.into());
            // self.csr_regs.write_raw(CSR_MEPC.into(), self.pc);
            // self.csr_regs.write_raw(CSR_MCAUSE.into(), trap_type.idx());
            // self.csr_regs.write_raw(CSR_MTVAL.into(), tval);

            self.csr_regs.xstatus.set(mstatus);
            self.csr_regs.mepc.set(self.pc);
            self.csr_regs.mcause.set(cause.into());
            self.csr_regs.mtval.set(tval);

            if let Some(sender) = &self.trace_sender {
                sender
                    .send(TraceType::Trap(trap_type, self.pc, tval))
                    .unwrap();
            };

            // let mtvec_val = self.csr_regs.read_raw(CSR_MTVEC.into());
            let mtvec = self.csr_regs.mtvec.get();
            self.npc = mtvec.get_trap_pc(trap_type);
            self.cur_priv.set(PrivilegeLevels::Machine);
        }
    }

    pub fn handle_interrupt(&mut self) {
        // read necessary csrs

        // let mip_val = self.csr_regs.read_raw(CSR_MIP.into());
        // let mie_val = self.csr_regs.read_raw(CSR_MIE.into());

        let xie = self.csr_regs.xie.get();
        let xip = self.csr_regs.xip.get();

        let mip_mie_val = u64::from(xie) & u64::from(xip);
        // no interupt allowed
        if mip_mie_val == 0 {
            return;
        }

        let mut mstatus = self.csr_regs.xstatus.get();

        let mideleg = self.csr_regs.mideleg.get();

        let m_a1 = mstatus.mie() & (self.cur_priv.get() == PrivilegeLevels::Machine);
        let m_a2 = self.cur_priv.get() < PrivilegeLevels::Machine;
        let int_to_m_enable = m_a1 | m_a2;
        let int_to_m_peding = mip_mie_val & !u64::from(mideleg);

        let s_a1 = mstatus.sie() & (self.cur_priv.get() == PrivilegeLevels::Supervisor);
        let s_a2 = self.cur_priv.get() < PrivilegeLevels::Supervisor;
        let int_to_s_enable = s_a1 | s_a2;
        let int_to_s_peding = mip_mie_val & u64::from(mideleg);

        // handing interupt in M mode
        if int_to_m_enable && int_to_m_peding != 0 {
            let cause = XipIn::from(int_to_m_peding).get_priority_interupt();
            mstatus.set_mpie(mstatus.mie());
            mstatus.set_mpp(self.cur_priv.get() as u8);
            mstatus.set_mie(false);
            // self.csr_regs.write_raw(CSR_MSTATUS.into(), mstatus.into());
            // self.csr_regs.write_raw(CSR_MEPC.into(), self.npc);
            // self.csr_regs.write_raw(CSR_MCAUSE.into(), cause.idx());
            // let mtvec_val = self.csr_regs.read_raw(CSR_MTVEC.into());

            self.csr_regs.xstatus.set(mstatus);
            self.csr_regs.mepc.set(self.npc);
            self.csr_regs.mcause.set(cause.idx().into());
            // self.csr_regs.mtval.set(0);

            let mtvec = self.csr_regs.mtvec.get();
            // todo! improve me
            self.npc = mtvec.get_trap_pc(cause);
            self.cur_priv.set(PrivilegeLevels::Machine);
        }
        // handing interupt in S mode
        // The sstatus register is a subset of the mstatus register.
        // In a straightforward implementation, reading or writing any field in sstatus is equivalent to
        // reading or writing the homonymous field in mstatus.
        // so, we use mstatus instead of sstatus below
        else if int_to_s_enable && int_to_s_peding != 0 {
            let cause = XipIn::from(int_to_s_peding).get_priority_interupt();
            // When a trap is taken, SPP is set to 0 if the trap originated from user mode, or 1 otherwise.
            mstatus.set_spp(!(self.cur_priv.get() == PrivilegeLevels::User));
            // When a trap is taken into supervisor mode, SPIE is set to SIE
            mstatus.set_spie(mstatus.sie());
            // and SIE is set to 0
            mstatus.set_sie(false);

            // self.csr_regs.write_raw(CSR_MSTATUS.into(), mstatus.into());
            // self.csr_regs.write_raw(CSR_SEPC.into(), self.npc);
            // self.csr_regs.write_raw(CSR_SCAUSE.into(), cause.idx());
            // let stvec_val = self.csr_regs.read_raw(CSR_STVEC.into());

            self.csr_regs.sepc.set(self.npc);
            self.csr_regs.scause.set(cause.idx().into());

            let stvec = self.csr_regs.stvec.get();
            self.npc = stvec.get_trap_pc(cause);
            self.cur_priv.set(PrivilegeLevels::Supervisor);
        }
    }

    pub fn read(&mut self, addr: u64, len: u64, access_type: AccessType) -> Result<u64, TrapType> {
        self.mmu.update_access_type(access_type);
        self.mmu.do_read(addr, len)
    }

    pub fn write(
        &mut self,
        addr: u64,
        data: u64,
        len: u64,
        access_type: AccessType,
    ) -> Result<u64, TrapType> {
        self.mmu.update_access_type(access_type);
        self.mmu.do_write(addr, data, len)
    }

    pub fn get_csr_regs(&mut self) -> &mut CsrRegs {
        &mut self.csr_regs
    }

    pub fn get_gp_regs(&mut self) -> &mut Gpr {
        &mut self.gpr
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
                    let tmp_data = self.mmu.bus.read(i, 4).unwrap();
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
        let data = self.mmu.bus.read(0x8000_1000, 8).unwrap();
        // !! must clear mem
        self.mmu.bus.write(0x8000_1000, 0, 8).unwrap();

        let cmd = FesvrCmd::from(data);
        if let Some(pass) = cmd.syscall_device() {
            if pass {
                self.cpu_state = CpuState::Stop;
            }
            // fail
            else {
                self.cpu_state = CpuState::Abort;
                println!("FAIL WITH EXIT CODE:{}", cmd.exit_code())
            }
        }
        cmd.character_device_write();
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
        let mut cpu = CpuCore::new(None);
        let mut dr = DeviceDram::new(128 * 1024 * 1024);
        dr.load_binary(file_name);

        let dram_u = DeviceType {
            start: 0x8000_0000,
            len: dr.capacity as u64,
            instance: dr.into(),
            name: "DRAM",
        };

        cpu.mmu.bus.add_device(dram_u);

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
