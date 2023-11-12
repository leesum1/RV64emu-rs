use core::cell::Cell;

use alloc::rc::Rc;
use log::{info, warn};

use crate::{
    config::Config,
    difftest::difftest_trait::Difftest,
    rv64core::bus::Bus,
    rv64core::csr_regs::CsrRegs,
    rv64core::csr_regs_define::XipIn,
    rv64core::gpr::Gpr,
    rv64core::inst::inst_base::{AccessType, PrivilegeLevels},
    rv64core::inst_decode::InstDecode,
    rv64core::traptype::TrapType,
    tools::{check_aligned, RcRefCell},
};

#[cfg(feature = "rv_debug_trace")]
use crate::trace::traces::TraceType;

use super::{
    cache::cache_system::CacheSystem, inst::inst_base::is_compressed_instruction, mmu::cpu_mmu::Mmu,
};

#[derive(PartialEq)]
pub enum CpuState {
    Running,
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
    pub config: Rc<Config>,
    #[cfg(feature = "rv_debug_trace")]
    pub trace_sender: Option<crossbeam_channel::Sender<TraceType>>,
}
unsafe impl Send for CpuCore {}
impl CpuCore {
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
            return Ok(u32::from_le_bytes(data_bytes) as u64);
        } else {
            return Err(TrapType::LoadAddressMisaligned(addr));
        }
    }
    pub fn inst_fetch(&mut self) -> Result<u64, TrapType> {
        self.pc = self.npc;

        // assert!(self.pc % 2 == 0, "pc must be aligned to 2");
        self.fetch_from_mem(self.pc, 4)
    }

    pub fn step(&mut self, inst: u32) -> Result<(), TrapType> {
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
        self.cache_system.borrow().show_perf();
        self.decode.show_perf();
        self.mmu.show_perf();
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
