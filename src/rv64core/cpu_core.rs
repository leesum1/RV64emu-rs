use core::{borrow::Borrow, cell::Cell, result};

use alloc::rc::Rc;
use log::{debug, info, warn};

use crate::{
    config::Config,
    dbg::dm_interface::DebugModuleSlave,
    difftest::difftest_trait::Difftest,
    rv64core::{
        bus::Bus,
        csr_regs::CsrRegs,
        csr_regs_define::XipIn,
        gpr::Gpr,
        inst::inst_base::{AccessType, PrivilegeLevels},
        inst_decode::InstDecode,
        traptype::TrapType,
    },
    tools::{check_aligned, RcRefCell},
};

#[cfg(feature = "rv_debug_trace")]
use crate::trace::traces::TraceType;

use super::{
    cache::cache_system::CacheSystem, inst::inst_base::is_compressed_instruction,
    mmu::cpu_mmu::Mmu, traptype::DebugCause,
};

pub struct DebugState {
    // 临时的状态位，dm 只负责置 1
    pub resumereq_flag: bool,
    pub singlestep_flag: bool,

    // 由 dm 控制置 1 或者清零
    pub resetreq_signal: bool,

    // 由 dm 控制置 1 或者清零
    pub haltreq_signal: bool,

    // 发送 resumereq 时，resumeack 首先被置为0, 进入 resume 状态
    // 处理器 resume 时，resumeack 被置为1
    pub resumeack: bool,

    // 处理器复位时，havereset被置为1
    // 通过往 ackhaveReset 写入 1 来清除
    pub havereset: bool,
    // 处理器是否处于 debug 模式
    pub debug_mode: bool,
}

impl DebugState {
    pub fn new() -> Self {
        DebugState {
            resumereq_flag: false,
            resetreq_signal: false,
            haltreq_signal: false,
            resumeack: false,
            havereset: false,
            debug_mode: false,
            singlestep_flag: false,
        }
    }
}

impl Default for DebugState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(PartialEq, Debug)]
pub enum CpuState {
    Running,
    Haltd,
    Stop,
    Abort,
}
pub struct CpuCoreBuild {
    hart_id: usize,
    shared_bus: RcRefCell<Bus>,
    config: Rc<Config>,
    boot_pc: u64,
    smode: bool,
    #[cfg(feature = "rv_debug_trace")]
    trace_sender: Option<crossbeam_channel::Sender<TraceType>>,
}
impl CpuCoreBuild {
    pub fn new(shared_bus: RcRefCell<Bus>, config: Rc<Config>) -> Self {
        CpuCoreBuild {
            hart_id: 0,
            shared_bus,
            config,
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
        let mut csr_regs_u = CsrRegs::new(self.hart_id, self.config.clone());
        let privi_u = Rc::new(Cell::new(PrivilegeLevels::Machine));
        // some csr regs are shared with other modules
        let xstatus = csr_regs_u.xstatus.clone();
        let satp = csr_regs_u.satp.clone();
        // let mtime = csr_regs_u.time.clone();
        let xip = csr_regs_u.xip.clone();

        let cache_system =
            RcRefCell::new(CacheSystem::new(self.shared_bus.clone(), self.config.clone()).into());

        let mmu_u = Mmu::new(
            cache_system.clone(),
            privi_u.clone(),
            xstatus,
            satp,
            self.config.clone(),
        );
        {
            let bus_u = mmu_u.caches.borrow_mut().bus.clone();
            let mut bus_u = bus_u.borrow_mut();

            let mtime = bus_u.clint.instance.add_hart(xip.clone());
            csr_regs_u.add_mtime(mtime);
            // add plic context for core0 m-mode and s-mode
            bus_u.plic.instance.add_context(xip.clone(), true);
            if self.smode {
                bus_u.plic.instance.add_context(xip, false);
            }
        }

        CpuCore {
            gpr: Gpr::new(),
            csr_regs: csr_regs_u,
            mmu: mmu_u,
            decode: InstDecode::new(self.config.clone()),
            cache_system,
            pc: self.boot_pc,
            npc: self.boot_pc,
            cur_priv: privi_u,
            cpu_state: CpuState::Stop,
            #[cfg(feature = "rv_debug_trace")]
            trace_sender: self.trace_sender.clone(),
            config: self.config.clone(),
            debug_state: DebugState::new(),
        }
    }
}

pub struct CpuCore {
    pub gpr: Gpr,
    pub csr_regs: CsrRegs,
    pub mmu: Mmu,
    pub decode: InstDecode,
    pub cache_system: RcRefCell<CacheSystem>,
    pub pc: u64,
    pub npc: u64,
    pub cur_priv: Rc<Cell<PrivilegeLevels>>,
    pub cpu_state: CpuState,
    pub debug_state: DebugState,
    pub config: Rc<Config>,
    #[cfg(feature = "rv_debug_trace")]
    pub trace_sender: Option<crossbeam_channel::Sender<TraceType>>,
}
impl CpuCore {
    fn reset(&mut self) {
        self.gpr = Gpr::new();
        self.csr_regs.reset();
        self.npc = 0x8000_0000; //TODO: config
        self.cpu_state = CpuState::Running;
        self.debug_state = DebugState::new();
        self.decode.reset();
        let mut cache = self.cache_system.borrow_mut();
        cache.icache.clear();
        cache.dcache.clear();

        debug!("cpu reset: pc:{:x}", self.npc);
    }

    fn fetch_from_mem(&mut self, addr: u64, size: u64) -> Result<u64, TrapType> {
        if check_aligned(addr, 4) {
            self.icahce_read(addr, 4)
        } else if self.config.is_enable_isa(b'c') {
            // Initialize data_bytes to store the bytes read from memory.
            let mut data_bytes = 0_u32.to_le_bytes();
            // Read two bytes at a time from memory and store them in data_bytes.
            for i in (0..size).step_by(2) {
                // Read a byte from memory.
                let byte = self.icahce_read(addr + i, 2)?;
                // Convert the byte to a byte array and copy it into data_bytes.
                let src = u16::to_le_bytes(byte as u16);
                let dest_range = i as usize..(i + 2) as usize;
                data_bytes[dest_range].copy_from_slice(&src[..]);

                if is_compressed_instruction(u32::from_le_bytes(data_bytes)) {
                    return Ok(u32::from_le_bytes(data_bytes) as u64);
                }
            }
            // Convert data_bytes to a u64 and return it.
            Ok(u32::from_le_bytes(data_bytes) as u64)
        } else {
            Err(TrapType::LoadAddressMisaligned(addr))
        }
    }
    pub fn inst_fetch(&mut self) -> Result<u64, TrapType> {
        self.pc = self.npc;

        // assert!(self.pc % 2 == 0, "pc must be aligned to 2");
        self.fetch_from_mem(self.pc, 4)
    }

    pub fn decode_and_excute(&mut self, inst: u32) -> Result<(), TrapType> {
        let inst_op = self.decode.fast_path(inst);
        match inst_op {
            Some(i) => {
                #[cfg(feature = "rv_debug_trace")]
                if let Some(sender) = &self.trace_sender {
                    sender.send(TraceType::Itrace(self.pc, inst)).unwrap();
                };
                (i.operation)(self, inst, self.pc) // return
            }
            None => {
                warn!("IllegalInstruction,pc:{:X},inst:{:x}", self.pc, inst);
                Err(TrapType::IllegalInstruction(inst.into()))
            }
        }
    }

    fn advance_pc(&mut self, inst: u32) {
        let is_rvc = is_compressed_instruction(inst);
        self.npc = self.pc.wrapping_add(if is_rvc { 2 } else { 4 });
    }
    pub fn show_perf(&self) {
        let cycle = self.csr_regs.cycle.get();
        let instret = self.csr_regs.instret.get();
        info!("cycle:{},instret:{}", cycle, instret);
        // let x = self.cache_system.borrow();
        // self.decode.show_perf();
        // self.mmu.show_perf();
    }

    fn set_pc(&mut self, pc: u64) {
        self.npc = pc;
    }

    pub fn resume_proc(&mut self) {
        assert!(self.debug_state.debug_mode, "not in debug mode");

        self.debug_state.resumereq_flag = false;
        debug!("exit debug mode");

        let dcsr = self.csr_regs.dcsr.get();
        // 1. pc changes to the value stored in dpc.
        self.set_pc(self.csr_regs.dpc.get());
        // 2. The current privilege mode and virtualization mode are changed to that specified by prv and v.
        // NOT support virtualization now
        self.cur_priv
            .set(PrivilegeLevels::from_usize(dcsr.prv().into()).unwrap());

        // 3. When resuming from debug mode, clear mstatus.MPRV if the new privilege mode is less than M-mode
        if (self.cur_priv.get() as usize) < (PrivilegeLevels::Machine as usize) {
            let mut xstatus_tmp = self.csr_regs.xstatus.get();
            xstatus_tmp.set_mprv(false);
            self.csr_regs.xstatus.set(xstatus_tmp)
        }

        // 4. The debug mode is cleared.
        self.debug_state.debug_mode = false;
        self.cpu_state = CpuState::Running;

        self.debug_state.resumeack = true;

        // 5. check step, and set single step flag
        if dcsr.step() {
            debug!("set single step flag");
            self.debug_state.singlestep_flag = true;
        }
    }

    fn single_step_proc(&mut self) {
        assert!(self.debug_state.singlestep_flag, "not in single step mode");
        debug!("single step");
        self.debug_state.singlestep_flag = false;

        // execute one instruction
        self.real_excute();
        // after execute one instruction, enter debug mode
        self.enter_debug_mode(DebugCause::Step, self.npc)
    }

    // fetch and execute one instruction
    fn real_excute(&mut self) {
        assert!(!self.debug_state.debug_mode, "in debug mode");
        assert_eq!(self.cpu_state, CpuState::Running, "not in running state");

        // Increment the cycle counter
        let cycle = self.csr_regs.cycle.get();
        self.csr_regs.cycle.set(cycle + 1);

        let fetch_ret = self.inst_fetch();
        let exe_ret = match fetch_ret {
            // op ret
            Ok(inst_val) => {
                self.advance_pc(inst_val as u32);
                self.decode_and_excute(inst_val as u32)
            }
            // fetch fault
            Err(trap_type) => Err(trap_type),
        };

        if let Err(trap_type) = exe_ret {
            self.handle_exceptions(trap_type);
        } else {
            // Increment the instruction counter
            let instret = self.csr_regs.instret.get();
            self.csr_regs.instret.set(instret + 1);
        }
    }

    pub fn execute(&mut self, num: usize) {
        for _ in 0..num {
            match self.cpu_state {
                CpuState::Running => {
                    if self.debug_state.resetreq_signal {
                        self.debug_state.havereset = true;
                        self.reset();
                    } else if self.debug_state.haltreq_signal {
                        self.enter_debug_mode(DebugCause::HaltReq, self.npc)
                    } else if self.debug_state.singlestep_flag {
                        self.single_step_proc();
                    } else {
                        self.real_excute();
                        self.handle_interrupt();
                    }
                }
                CpuState::Haltd => {
                    if self.debug_state.resumereq_flag {
                        self.resume_proc();
                    }
                }
                _ => break,
            };
        }
    }

    // for difftest
    pub fn execute_as_ref(&mut self, num: usize) {
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
                            self.decode_and_excute(inst_val as u32)
                        }
                        // fetch fault
                        Err(trap_type) => Err(trap_type),
                    };

                    if let Err(trap_type) = exe_ret {
                        self.handle_exceptions(trap_type);
                        continue;
                    }
                    // self.handle_interrupt();
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
            info!("GOOD TRAP");
        } else {
            info!("BAD TRAP");
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

        log::debug!(
            "pc:{:x},trap_type:{:?},cause:{:?},tval:{:x}",
            self.pc,
            trap_type,
            cause,
            tval
        );

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

            log::trace!("mmode int pc:{:x},cause:{:?}", self.pc, cause,);

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

            log::trace!("smode int pc:{:x},cause:{:?}", self.pc, cause,);

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

    pub fn read(
        &mut self,
        addr: u64,
        len: usize,
        access_type: AccessType,
    ) -> Result<u64, TrapType> {
        self.mmu.update_access_type(&access_type);
        let paddr = self.mmu.translate(addr, len)?;
        match self.cache_system.borrow_mut().dcache.read(paddr, len) {
            Ok(data) => Ok(data),
            Err(_err) => Err(access_type.throw_access_exception()),
        }
    }

    pub fn icahce_read(&mut self, addr: u64, len: usize) -> Result<u64, TrapType> {
        let access_type = AccessType::Fetch(addr);
        self.mmu.update_access_type(&access_type);
        let paddr = self.mmu.translate(addr, len)?;

        assert_ne!(len, 0, "icache read len is zero");
        match self.cache_system.borrow_mut().icache.read(paddr, len) {
            Ok(data) => Ok(data),
            Err(_err) => Err(access_type.throw_access_exception()),
        }
    }

    pub fn write(
        &mut self,
        addr: u64,
        data: u64,
        len: usize,
        access_type: AccessType,
    ) -> Result<u64, TrapType> {
        self.mmu.update_access_type(&access_type);
        let paddr = self.mmu.translate(addr, len)?;
        match self
            .cache_system
            .borrow_mut()
            .dcache
            .write(paddr, data, len)
        {
            Ok(data) => Ok(data),
            Err(_err) => Err(access_type.throw_access_exception()),
        }
    }

    pub fn lr_sc_reservation_set(&mut self, addr: u64) {
        self.mmu
            .caches
            .borrow_mut()
            .bus
            .borrow_mut()
            .lr_sc_set
            .set(addr);
    }
    pub fn lr_sc_reservation_check_and_clear(&mut self, addr: u64) -> bool {
        self.mmu
            .caches
            .borrow_mut()
            .bus
            .borrow_mut()
            .lr_sc_set
            .check_and_clear(addr)
    }
    // pub fn lr_sc_reservation_clear(&mut self) {
    //     self.lr_sc_set.lock().unwrap().clear();
    // }

    // halt
    pub fn enter_debug_mode(&mut self, cause: DebugCause, pc: u64) {
        debug!("enter debug mode,cause:{:?},pc:{:x}", cause, pc);

        let mut dcsr = self.csr_regs.dcsr.get();

        // 1. dcsr->cause is updated.
        dcsr.set_cause(cause as u8);
        // 2. dcsr->prv and dcsr->v are set to reflect current privilege mode.
        dcsr.set_prv(self.cur_priv.get() as u8);
        dcsr.set_v(false); // do not support virtualnization

        self.csr_regs.dcsr.set(dcsr);

        // 3. dpc is set to the next instruction that should be executed.

        self.csr_regs.dpc.set(pc);
        // 4. The hart enters Debug Mode.
        self.debug_state.debug_mode = true;
        self.cpu_state = CpuState::Haltd;

        // 5. debug mode is always performed in M mode
        self.cur_priv.set(PrivilegeLevels::Machine);
    }
}

impl Difftest for CpuCore {
    fn set_csr(&mut self, csr: u64, val: u64) {
        self.csr_regs.write_raw(csr, val);
    }
    fn get_csr(&mut self, csr: u64) -> u64 {
        self.csr_regs.read_raw(csr)
    }
    fn set_mem(&mut self, paddr: u64, data: u64, len: usize) {
        let _ret = self.mmu.caches.borrow_mut().dcache.write(paddr, data, len);
    }
    fn get_mem(&self, paddr: u64, len: usize) -> u64 {
        self.mmu
            .caches
            .borrow_mut()
            .dcache
            .read(paddr, len)
            .map_or(0, |x| x)
    }
    fn get_pc(&self) -> u64 {
        self.pc
    }
    fn set_pc(&mut self, pc: u64) {
        self.npc = pc;
    }
    fn raise_intr(&mut self, irq_num: u64) {
        let mut xip = self.csr_regs.xip.get();
        xip.set_irq(irq_num as usize);
        self.csr_regs.xip.set(xip);
        self.handle_interrupt();
    }

    fn set_reg(&mut self, idx: usize, val: u64) {
        assert!(idx < 32, "idx must be less than 32");
        self.gpr.write(idx as u64, val)
    }

    fn get_reg(&self, idx: usize) -> u64 {
        assert!(idx < 32, "idx must be less than 32");
        self.gpr.read(idx as u64)
    }
}

impl DebugModuleSlave for CpuCore {
    fn read_gpr(&mut self, regno: usize) -> u64 {
        let val = self.gpr.read(regno as u64);
        debug!("[DebugModuleSlave] read gpr[{}]:{:x}", regno, val);
        val
    }

    fn write_gpr(&mut self, regno: usize, value: u64) {
        self.gpr.write(regno as u64, value);
        debug!("[DebugModuleSlave] write gpr[{}]:{:x}", regno, value);
    }

    fn read_memory(&mut self, address: u64, length: usize) -> Option<u64> {
        let paddr = address;
        let result = match self.cache_system.borrow_mut().dcache.read(paddr, length) {
            Ok(data) => Some(data),
            Err(_err) => None,
        };
        debug!(
            "[DebugModuleSlave] read memory address:{:x},length:{},value:{:x?}",
            address, length, result
        );
        result
    }

    fn write_memory(&mut self, address: u64, length: usize, value: u64) -> Option<u64> {
        let paddr = address;
        let result = match self
            .cache_system
            .borrow_mut()
            .dcache
            .write(paddr, value, length)
        {
            Ok(data) => Some(data),
            Err(_err) => None,
        };
        debug!(
            "[DebugModuleSlave] write memory address:{:x},length:{},value:{:x?}",
            address, length, value
        );
        result
    }

    fn read_csr(&mut self, csr_addr: usize) -> u64 {
        let val = self.csr_regs.read_raw(csr_addr as u64);
        debug!("[DebugModuleSlave] read csr[{:x}]:{:x}", csr_addr, val);
        val
    }

    fn write_csr(&mut self, csr_addr: usize, value: u64) {
        self.csr_regs.write_raw(csr_addr as u64, value);
        debug!("[DebugModuleSlave] write csr[{:x}]:{:x}", csr_addr, value);
    }

    fn set_haltreq(&mut self, val: bool) {
        self.debug_state.haltreq_signal = val;
    }

    fn resumereq(&mut self) {
        self.debug_state.resumereq_flag = true;
        self.debug_state.resumeack = false;
        debug!("[DebugModuleSlave] resumereq");
    }

    fn halted(&mut self) -> bool {
        self.cpu_state == CpuState::Haltd
    }

    fn resume_ack(&mut self) -> bool {
        self.debug_state.resumeack
    }

    fn set_reset_req(&mut self, val: bool) {
        self.debug_state.resetreq_signal = val;
    }

    fn havereset(&mut self) -> bool {
        self.debug_state.havereset
    }

    fn clear_havereset(&mut self) {
        debug!("[DebugModuleSlave] clear havereset");
        self.debug_state.havereset = false;
    }
}
