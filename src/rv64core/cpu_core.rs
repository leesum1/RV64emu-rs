use core::cell::Cell;

use std::{
    fs::File,
    io::Write,
    rc::Rc,
    sync::{Arc, Mutex},
};

use log::warn;

use crate::{
    difftest::difftest_trait::Difftest,
    rv64core::bus::Bus,
    rv64core::cpu_icache::CpuIcache,
    rv64core::csr_regs::CsrRegs,
    rv64core::csr_regs_define::XipIn,
    rv64core::gpr::Gpr,
    rv64core::inst::{
        inst_base::{AccessType, FesvrCmd, PrivilegeLevels},
        inst_rv64a::LrScReservation,
    },
    rv64core::inst_decode::InstDecode,
    rv64core::traptype::TrapType,
};

#[cfg(feature = "rv_debug_trace")]
use crate::trace::traces::TraceType;

use super::{inst::inst_base::check_aligned, mmu::mmu::Mmu};

#[derive(PartialEq)]
pub enum CpuState {
    Running,
    Stop,
    Abort,
}
pub struct CpuCoreBuild {
    hart_id: usize,
    shared_bus: Arc<Mutex<Bus>>,
    boot_pc: u64,
    smode: bool,
    #[cfg(feature = "rv_debug_trace")]
    trace_sender: Option<crossbeam_channel::Sender<TraceType>>,
}
impl CpuCoreBuild {
    pub fn new(shared_bus: Arc<Mutex<Bus>>) -> Self {
        CpuCoreBuild {
            hart_id: 0,
            shared_bus,
            boot_pc: 0x8000_0000,
            #[cfg(feature = "rv_debug_trace")]
            trace_sender: None,
            smode: true,
        }
    }
    pub fn with_boot_pc(&mut self, boot_pc: u64) -> &mut Self {
        self.boot_pc = boot_pc;
        self
    }
    #[cfg(feature = "rv_debug_trace")]
    pub fn with_trace(&mut self, trace_sender: crossbeam_channel::Sender<TraceType>) -> &mut Self {
        self.trace_sender = Some(trace_sender);
        self
    }
    pub fn with_hart_id(&mut self, hart_id: usize) -> &mut Self {
        self.hart_id = hart_id;
        self
    }
    pub fn with_smode(&mut self, smode: bool) -> &mut Self {
        self.smode = smode;
        self
    }

    pub fn build(&self) -> CpuCore {
        let mut csr_regs_u = CsrRegs::new(self.hart_id);
        let privi_u = Rc::new(Cell::new(PrivilegeLevels::Machine));
        // some csr regs are shared with other modules
        let xstatus = csr_regs_u.xstatus.clone();
        let satp = csr_regs_u.satp.clone();
        // let mtime = csr_regs_u.time.clone();
        let xip = csr_regs_u.xip.clone();

        let mmu_u = Mmu::new(self.shared_bus.clone(), privi_u.clone(), xstatus, satp);
        {
            let mut bus_u = mmu_u.bus.lock().unwrap();
            let mtime = bus_u.clint.instance.add_hart(xip.clone());
            csr_regs_u.add_mtime(mtime);
            // add plic context for core0 m-mode and s-mode
            bus_u.plic.instance.add_context(xip.clone(), true);
            if self.smode {
                bus_u.plic.instance.add_context(xip, false);
            }
        }

        let share_lr_sc_set = mmu_u.bus.lock().unwrap().lr_sc_set.clone();
        let share_amo = mmu_u.bus.lock().unwrap().amo_mutex.clone();
        CpuCore {
            gpr: Gpr::new(),
            csr_regs: csr_regs_u,
            mmu: mmu_u,
            decode: InstDecode::new(),
            pc: self.boot_pc,
            npc: self.boot_pc,
            cur_priv: privi_u,
            lr_sc_set: share_lr_sc_set,
            cpu_state: CpuState::Stop,
            cpu_icache: CpuIcache::new(),
            #[cfg(feature = "rv_debug_trace")]
            trace_sender: self.trace_sender.clone(),
            amo_mutex: share_amo,
        }
    }
}

pub struct CpuCore {
    pub gpr: Gpr,
    pub csr_regs: CsrRegs,
    pub mmu: Mmu,
    pub decode: InstDecode,
    pub pc: u64,
    pub npc: u64,
    pub cur_priv: Rc<Cell<PrivilegeLevels>>,
    // todo! move to bus
    pub lr_sc_set: Arc<Mutex<LrScReservation>>, // for rv64a inst
    pub amo_mutex: Arc<Mutex<()>>,
    pub cpu_state: CpuState,
    pub cpu_icache: CpuIcache,
    #[cfg(feature = "rv_debug_trace")]
    pub trace_sender: Option<crossbeam_channel::Sender<TraceType>>,
}
unsafe impl Send for CpuCore {}
impl CpuCore {
    fn fetch_from_mem(&mut self, addr: u64, size: u64) -> Result<u64, TrapType> {
        // data_bytes will contain the bytes read from memory.
        let mut data_bytes = 0_u32.to_le_bytes();
        // We read two bytes at a time, so we step by 2.
        for i in (0..size).step_by(2) {
            // Read a byte from memory.
            let byte = self.read(addr + i, 2, AccessType::Fetch(addr + i))?;
            // Convert the byte to a byte array.
            let src = byte.to_le_bytes();
            // Copy the byte into data_bytes.
            data_bytes[(i as usize)..(i as usize + 2)].copy_from_slice(&src[..2]);
        }
        // Convert data_bytes to a u64 and return it.
        Ok(u32::from_le_bytes(data_bytes) as u64)
    }
    pub fn inst_fetch(&mut self) -> Result<u64, TrapType> {
        self.pc = self.npc;

        // first lookup icache
        // if icache hit ,than return,
        // else load inst from mem and push into icache
        #[cfg(feature = "inst_cache")]
        match self.cpu_icache.get_inst(self.pc) {
            Some(icache_inst) => {
                self.cpu_icache.insert_inst(self.pc, icache_inst);
                Ok(icache_inst as u64)
            }
            None => self.fetch_from_mem(self.pc, 4),
        }

        #[cfg(not(feature = "inst_cache"))]
        self.fetch_from_mem(self.pc, 4)
    }

    pub fn step(&mut self, inst: u32) -> Result<(), TrapType> {
        let inst_op = if cfg!(feature = "decode_cache") {
            let decode_cache_hit = self.decode.fast_path(inst);
            match decode_cache_hit {
                Some(operation) => Some(operation),
                None => self.decode.slow_path(inst),
            }
        } else {
            self.decode.slow_path(inst)
        };

        match inst_op {
            Some(i) => {
                #[cfg(feature = "rv_debug_trace")]
                if let Some(sender) = &self.trace_sender {
                    sender.send(TraceType::Itrace(self.pc, inst)).unwrap();
                };
                (i.operation)(self, inst, self.pc) // return
            }
            None => {
                warn!("inst err,pc:{:X},inst:{:x}", self.pc, inst);
                Err(TrapType::IllegalInstruction(inst.into()))
            }
        }
    }

    fn advance_pc(&mut self, inst: u32) {
        let is_rvc = inst & 0x3 != 0x3;
        self.npc = self.pc.wrapping_add(if is_rvc { 2 } else { 4 });
    }

    pub fn execute(&mut self, num: usize) {
        for _ in 0..num {
            match self.cpu_state {
                CpuState::Running => {
                    // Increment the cycle counter
                    let cycle = self.csr_regs.cycle.get();
                    self.csr_regs.cycle.set(cycle + 1);

                    let fetch_ret = self.inst_fetch();
                    let exe_ret = match fetch_ret {
                        // op ret
                        Ok(inst_val) => {
                            self.advance_pc(inst_val as u32);
                            self.step(inst_val as u32)
                        }
                        // fetch fault
                        Err(trap_type) => Err(trap_type),
                    };

                    if let Err(trap_type) = exe_ret {
                        self.handle_exceptions(trap_type);
                        continue;
                    }

                    self.handle_interrupt();
                    // self.mmu.bus.lock().unwrap().update();

                    // Increment the instruction counter
                    let instret = self.csr_regs.instret.get();
                    self.csr_regs.instret.set(instret + 1);
                }
                _ => break,
            };
        }
    }

    #[cfg(feature = "support_am")]
    pub fn halt(&mut self) -> usize {
        let a0 = self.gpr.read_by_name("a0");

        if let 0 = a0 {
            warn!("GOOD TRAP");
        } else {
            warn!("BAD TRAP");
        }
        self.cpu_state = CpuState::Stop;
        a0 as usize
    }

    pub fn handle_exceptions(&mut self, trap_type: TrapType) {
        let medeleg = self.csr_regs.medeleg.get();
        let mut mstatus = self.csr_regs.xstatus.get();

        let has_exception = u64::from(medeleg) & (1_u64 << trap_type.get_exception_num()) != 0;

        let trap_to_s_enable = self.cur_priv.get() <= PrivilegeLevels::Supervisor;

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

            self.csr_regs.xstatus.set(mstatus);
            self.csr_regs.sepc.set(self.pc);
            self.csr_regs.scause.set(cause.into());
            self.csr_regs.stval.set(tval);
            #[cfg(feature = "rv_debug_trace")]
            if let Some(sender) = &self.trace_sender {
                sender
                    .send(TraceType::Trap(trap_type, self.pc, tval))
                    .unwrap();
            };

            let stvec = self.csr_regs.stvec.get();

            self.npc = stvec.get_trap_pc(trap_type);
            self.cur_priv.set(PrivilegeLevels::Supervisor);
        }
        // exception to M mode
        else {
            mstatus.set_mpie(mstatus.mie());
            mstatus.set_mie(false);
            mstatus.set_mpp(self.cur_priv.get() as u8);

            self.csr_regs.xstatus.set(mstatus);
            self.csr_regs.mepc.set(self.pc);
            self.csr_regs.mcause.set(cause.into());
            self.csr_regs.mtval.set(tval);
            #[cfg(feature = "rv_debug_trace")]
            if let Some(sender) = &self.trace_sender {
                sender
                    .send(TraceType::Trap(trap_type, self.pc, tval))
                    .unwrap();
            };

            let mtvec = self.csr_regs.mtvec.get();
            self.npc = mtvec.get_trap_pc(trap_type);
            self.cur_priv.set(PrivilegeLevels::Machine);
        }
    }

    pub fn handle_interrupt(&mut self) {
        // read necessary csrs

        let xie = self.csr_regs.xie.get();
        let xip = self.csr_regs.xip.get();

        let mip_mie_val = u64::from(xie) & u64::from(xip);
        // no interupt allowed
        if mip_mie_val == 0 {
            return;
        }
        // warn!("mip_mie_val:{:?}", XieIn::from(mip_mie_val));
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

            self.csr_regs.xstatus.set(mstatus);
            self.csr_regs.mepc.set(self.npc);
            self.csr_regs.mcause.set(cause.idx().into());
            // self.csr_regs.mtval.set(0);
            #[cfg(feature = "rv_debug_trace")]
            if let Some(sender) = &self.trace_sender {
                sender.send(TraceType::Trap(cause, self.pc, 0)).unwrap();
            };

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
            #[cfg(feature = "rv_debug_trace")]
            if let Some(sender) = &self.trace_sender {
                sender.send(TraceType::Trap(cause, self.pc, 0)).unwrap();
            };

            self.csr_regs.xstatus.set(mstatus);
            self.csr_regs.sepc.set(self.npc);
            self.csr_regs.scause.set(cause.idx().into());

            let stvec = self.csr_regs.stvec.get();
            self.cur_priv.set(PrivilegeLevels::Supervisor);
            self.npc = stvec.get_trap_pc(cause);
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

    pub fn lr_sc_reservation_set(&mut self, addr: u64) {
        self.lr_sc_set.lock().unwrap().set(addr);
    }
    pub fn lr_sc_reservation_check_and_clear(&mut self, addr: u64) -> bool {
        self.lr_sc_set.lock().unwrap().check_and_clear(addr)
    }
    // pub fn lr_sc_reservation_clear(&mut self) {
    //     self.lr_sc_set.lock().unwrap().clear();
    // }

    // for riscof
    pub fn dump_signature(&mut self, file_name: &str) {
        let fd = File::create(file_name);

        let sig_start = self.gpr.read_by_name("a1");
        let sig_end = self.gpr.read_by_name("a2");

        fd.map_or_else(
            |err| warn!("{err}"),
            |mut file| {
                let mut bus_u = self.mmu.bus.lock().unwrap();
                for i in (sig_start..sig_end).step_by(4) {
                    let tmp_data = bus_u.read(i, 4).unwrap();
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
        let mut bus_u = self.mmu.bus.lock().unwrap();

        let data = bus_u.read(0x8000_1000, 8).unwrap();
        // !! must clear mem
        bus_u.write(0x8000_1000, 0, 8).unwrap();

        let cmd = FesvrCmd::from(data);
        if let Some(pass) = cmd.syscall_device() {
            if pass {
                self.cpu_state = CpuState::Stop;
            }
            // fail
            else {
                self.cpu_state = CpuState::Abort;
                warn!("FAIL WITH EXIT CODE:{}", cmd.exit_code())
            }
        }
        cmd.character_device_write();
    }
}

impl Difftest for CpuCore {
    fn set_regs(&mut self, regs: &[u64; 32]) {
        for idx in 0..32 {
            self.gpr.write(idx, regs[idx as usize]);
        }
    }
    fn get_regs(&mut self) -> [u64; 32] {
        let mut regs = [0u64; 32];
        for idx in 0..32 {
            regs[idx as usize] = self.gpr.read(idx);
        }
        regs
    }
    fn set_csr(&mut self, csr: u64, val: u64) {
        self.csr_regs.write_raw(csr, val);
    }

    fn get_csr(&mut self, csr: u64) -> u64 {
        self.csr_regs.read_raw(csr)
    }

    fn set_mem(&mut self, paddr: u64, data: u64, len: usize) {
        let _ret = self.mmu.bus.lock().unwrap().write(paddr, data, len);
    }

    fn get_mem(&mut self, paddr: u64, len: usize) -> u64 {
        self.mmu
            .bus
            .lock()
            .unwrap()
            .read(paddr, len)
            .map_or(0, |x| x)
    }

    fn get_pc(&mut self) -> u64 {
        self.npc
    }
    fn set_pc(&mut self, pc: u64) {
        self.npc = pc;
    }
    fn raise_intr(&mut self, _irq: u64) {
        todo!()
    }
}
