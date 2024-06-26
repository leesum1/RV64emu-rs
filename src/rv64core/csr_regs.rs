use core::cell::Cell;

use alloc::rc::Rc;
use hashbrown::HashMap;

use crate::{
    config::Config,
    rv64core::csr_regs_define::{
        CommonCSR, Counter, Csr, CsrEnum, Medeleg, MedelegIn, Mideleg, MidelegIn, Misa,
        ReadOnlyCSR, Satp, SatpIn, Xcause, XcauseIn, Xie, XieIn, Xip, XipIn, Xstatus, XstatusIn,
        Xtvec, XtvecIn,
    },
    rv64core::inst::inst_base::{
        AccessType, PrivilegeLevels, CSR_CYCLE, CSR_INSTRET, CSR_MARCHID, CSR_MCAUSE,
        CSR_MCOUNTEREN, CSR_MCYCLE, CSR_MEDELEG, CSR_MEPC, CSR_MHARTID, CSR_MIDELEG, CSR_MIE,
        CSR_MIMPID, CSR_MINSTRET, CSR_MIP, CSR_MISA, CSR_MSCRATCH, CSR_MSTATUS, CSR_MTVAL,
        CSR_MTVEC, CSR_MVENDORID, CSR_SATP, CSR_SCAUSE, CSR_SCOUNTEREN, CSR_SEPC, CSR_SIE, CSR_SIP,
        CSR_SSCRATCH, CSR_SSTATUS, CSR_STVAL, CSR_STVEC, CSR_TIME, CSR_TSELECT, MASK_ALL,
    },
    rv64core::traptype::TrapType,
    tools::RcCell,
};

use super::{
    csr_regs_define::{Dcsr, DcsrIn},
    inst::inst_base::{CSR_DCSR, CSR_DPC, CSR_DSCRATCH0, CSR_DSCRATCH1},
};

pub struct CsrRegs {
    config: Rc<Config>,
    pub csr_map: HashMap<u64, CsrEnum>,
    pub cur_priv: PrivilegeLevels,
    pub xstatus: RcCell<XstatusIn>,
    pub xip: RcCell<XipIn>,
    pub xie: RcCell<XieIn>,
    pub mtvec: RcCell<XtvecIn>,
    pub stvec: RcCell<XtvecIn>,
    pub mcause: RcCell<XcauseIn>,
    pub scause: RcCell<XcauseIn>,
    pub medeleg: RcCell<MedelegIn>,
    pub mideleg: RcCell<MidelegIn>,
    pub mepc: RcCell<u64>,
    pub sepc: RcCell<u64>,
    pub satp: RcCell<SatpIn>,
    pub mtval: RcCell<u64>,
    pub stval: RcCell<u64>,
    pub cycle: RcCell<u64>,
    pub instret: RcCell<u64>,

    // debug mode
    pub dcsr: RcCell<DcsrIn>,
    pub dpc: RcCell<u64>,
}

impl CsrRegs {
    pub fn reset(&mut self) {
        let mut mstatus_val = XstatusIn::new().with_mpp(PrivilegeLevels::Machine as u8);

        mstatus_val.set_mbe(false);
        mstatus_val.set_sbe(false);
        mstatus_val.set_ube(false);
        if self.config.s_mode() {
            mstatus_val.set_sxl(2) // 64
        }
        if self.config.u_mode() {
            mstatus_val.set_uxl(2); // 64
            mstatus_val.set_mprv(false);
        }
        self.xstatus.set(mstatus_val);

        self.xip.set(XipIn::new());
        self.xie.set(XieIn::new());
        self.mtvec.set(XtvecIn::new());
        self.stvec.set(XtvecIn::new());
        self.mcause.set(XcauseIn::new());
        self.scause.set(XcauseIn::new());
        self.medeleg.set(MedelegIn::new());
        self.mideleg.set(MidelegIn::new());
        self.mepc.set(0);
        self.sepc.set(0);
        self.satp.set(SatpIn::new());
        self.mtval.set(0);
        self.stval.set(0);
        self.cycle.set(0);
        self.instret.set(0);
        self.dcsr
            .set(DcsrIn::new().with_debugver(4).with_mprven(true));
        self.dpc.set(0);
    }

    pub fn new(hart_id: usize, config: Rc<Config>) -> Self {
        let mut misa_val = Misa::new().with_i(true).with_mxl(2); // 64

        if config.is_enable_isa(b'm') {
            misa_val.set_m(true);
        }
        if config.is_enable_isa(b'a') {
            misa_val.set_a(true);
        }
        if config.is_enable_isa(b'c') {
            misa_val.set_c(true);
        }
        if config.s_mode() {
            misa_val.set_s(true);
        }
        if config.u_mode() {
            misa_val.set_u(true);
        }

        let mut mstatus_val = XstatusIn::new().with_mpp(PrivilegeLevels::Machine as u8);

        mstatus_val.set_mbe(false);
        mstatus_val.set_sbe(false);
        mstatus_val.set_ube(false);
        if config.s_mode() {
            mstatus_val.set_sxl(misa_val.mxl())
        }
        if config.u_mode() {
            mstatus_val.set_uxl(misa_val.mxl());
            mstatus_val.set_mprv(false);
        }

        let mut mstatus_rmask = XstatusIn::new();

        // only support little endian
        mstatus_rmask.set_sbe(true);
        mstatus_rmask.set_mbe(true);
        mstatus_rmask.set_ube(true);

        if !config.s_mode() {
            mstatus_rmask.set_mxr(true);
            mstatus_rmask.set_sum(true);
            mstatus_rmask.set_tvm(true);
            mstatus_rmask.set_tsr(true);
        }

        if !config.u_mode() {
            mstatus_rmask.set_mprv(true);
        }

        if !config.u_mode() && !config.s_mode() {
            mstatus_rmask.set_tw(true);
        }
        if !config.is_enable_isa(b'f') {
            mstatus_rmask.set_fs(0b11);
            mstatus_rmask.set_vs(0b11);
            mstatus_rmask.set_xs(0b11);
            mstatus_rmask.set_sd(true);
        }

        let mstatus_rmask = !u64::from(mstatus_rmask);

        let sstatus_wmask = u64::from(
            XstatusIn::new()
                .with_spp(true)
                .with_sie(true)
                .with_spie(true)
                .with_ube(true)
                .with_vs(0b11)
                .with_fs(0b11)
                .with_xs(0b11)
                .with_sum(true)
                .with_mxr(true)
                .with_sd(true),
        ) & mstatus_rmask;

        let mstatus_wmask = XstatusIn::from(mstatus_rmask).with_uxl(0).with_sxl(0);

        // read only
        let misa = misa_val;
        let mhartid = ReadOnlyCSR(hart_id as u64);
        let marchid = CommonCSR::new_noshare(0);
        let mvendorid = CommonCSR::new_noshare(0);
        let mimpid = CommonCSR::new_noshare(0);
        // important csrs
        let xstatus_share = RcCell::new(mstatus_val.into());
        let mstatus = Xstatus::new(xstatus_share.clone(), mstatus_rmask, mstatus_wmask.into());
        let sstatus = Xstatus::new(xstatus_share.clone(), mstatus_rmask, sstatus_wmask);

        let sip_mask = XieIn::new().with_seie(true).with_ssie(true).with_stie(true);

        let xip_share = Rc::new(Cell::new(XipIn::new()));
        let mip = Xip::new(xip_share.clone(), MASK_ALL);
        let sip = Xip::new(xip_share.clone(), sip_mask.into());

        let xie_share = Rc::new(Cell::new(XieIn::new()));
        let mie = Xie::new(xie_share.clone(), MASK_ALL);
        let sie = Xie::new(xie_share.clone(), sip_mask.into());

        let mcause_share = Rc::new(Cell::new(XcauseIn::new()));
        let mcause = Xcause::new(mcause_share.clone());
        let scause_share = Rc::new(Cell::new(XcauseIn::new()));
        let scause = Xcause::new(scause_share.clone());

        let mtvec_share = Rc::new(Cell::new(XtvecIn::new()));
        let mtvec = Xtvec::new(mtvec_share.clone());
        let stvec_share = Rc::new(Cell::new(XtvecIn::new()));
        let stvec = Xtvec::new(stvec_share.clone());

        let medeleg_share = Rc::new(Cell::new(MedelegIn::new()));
        let medeleg = Medeleg::new(medeleg_share.clone());
        let mideleg_share = Rc::new(Cell::new(MidelegIn::new()));
        let mideleg = Mideleg::new(mideleg_share.clone(), MASK_ALL);

        let mepc_share = Rc::new(Cell::new(0_u64));
        let mepc = CommonCSR::new(mepc_share.clone());
        let sepc_share = Rc::new(Cell::new(0_u64));
        let sepc = CommonCSR::new(sepc_share.clone());
        let mtval_share = Rc::new(Cell::new(0_u64));
        let mtval = CommonCSR::new(mtval_share.clone());
        let stval_share = Rc::new(Cell::new(0_u64));
        let stval = CommonCSR::new(stval_share.clone());
        let mscratch = CommonCSR::new_noshare(0);
        let sscratch = CommonCSR::new_noshare(0);

        let satp_share = Rc::new(Cell::new(SatpIn::new()));
        let satp = Satp::new(
            satp_share.clone(),
            xstatus_share.clone(),
            config.get_mmu_type(),
        );

        let cycle_share = Rc::new(Cell::new(0));
        let mcycle = Counter::new(cycle_share.clone());
        let cycle = Counter::new(cycle_share.clone());

        let instret_share = Rc::new(Cell::new(0));
        let minstret = Counter::new(instret_share.clone());
        let instret = Counter::new(instret_share.clone());

        let mcounteren_share = Rc::new(Cell::new(0));
        let scounteren_share = Rc::new(Cell::new(0));
        let mcounteren = CommonCSR::new(mcounteren_share);
        let scounteren = CommonCSR::new(scounteren_share);

        // debug mode
        let dcsr_share = Rc::new(Cell::new(DcsrIn::new().with_debugver(4).with_mprven(true)));
        let dpc_share = Rc::new(Cell::new(0));
        let dpc = CommonCSR::new(dpc_share.clone());
        let dcsr = Dcsr::new(dcsr_share.clone());
        let dscratch0 = CommonCSR::new_noshare(0);
        let dscratch1 = CommonCSR::new_noshare(0);

        // not support hardware trigger now
        let tselect = ReadOnlyCSR(0);
        let tdata1 = ReadOnlyCSR(0);
        let tdata2 = ReadOnlyCSR(0);
        let tdata3 = ReadOnlyCSR(0);
        let tinfo = ReadOnlyCSR(0);


        let mut csr_map: HashMap<u64, CsrEnum> = HashMap::new();

        csr_map.insert(CSR_MISA.into(), misa.into());
        csr_map.insert(CSR_MHARTID.into(), mhartid.into());
        csr_map.insert(CSR_MARCHID.into(), marchid.into());
        csr_map.insert(CSR_MVENDORID.into(), mvendorid.into());
        csr_map.insert(CSR_MIMPID.into(), mimpid.into());
        csr_map.insert(CSR_MSTATUS.into(), mstatus.into());
        csr_map.insert(CSR_SSTATUS.into(), sstatus.into());
        csr_map.insert(CSR_MIP.into(), mip.into());
        csr_map.insert(CSR_SIP.into(), sip.into());
        csr_map.insert(CSR_MIE.into(), mie.into());
        csr_map.insert(CSR_SIE.into(), sie.into());
        csr_map.insert(CSR_MCAUSE.into(), mcause.into());
        csr_map.insert(CSR_SCAUSE.into(), scause.into());
        csr_map.insert(CSR_MTVEC.into(), mtvec.into());
        csr_map.insert(CSR_STVEC.into(), stvec.into());
        csr_map.insert(CSR_MEDELEG.into(), medeleg.into());
        csr_map.insert(CSR_MIDELEG.into(), mideleg.into());
        csr_map.insert(CSR_MEPC.into(), mepc.into());
        csr_map.insert(CSR_SEPC.into(), sepc.into());
        csr_map.insert(CSR_MTVAL.into(), mtval.into());
        csr_map.insert(CSR_STVAL.into(), stval.into());
        csr_map.insert(CSR_SATP.into(), satp.into());
        csr_map.insert(CSR_MSCRATCH.into(), mscratch.into());
        csr_map.insert(CSR_SSCRATCH.into(), sscratch.into());
        csr_map.insert(CSR_MCYCLE.into(), mcycle.into());
        csr_map.insert(CSR_CYCLE.into(), cycle.into());
        // csr_map.insert(CSR_MTIME.into(), mtime.into());
        // csr_map.insert(CSR_TIME.into(), time.into());
        csr_map.insert(CSR_MINSTRET.into(), minstret.into());
        csr_map.insert(CSR_INSTRET.into(), instret.into());
        csr_map.insert(CSR_MCOUNTEREN.into(), mcounteren.into());
        csr_map.insert(CSR_SCOUNTEREN.into(), scounteren.into());
        csr_map.insert(CSR_TSELECT.into(), tselect.into());

        // debug mode
        csr_map.insert(CSR_DCSR.into(), dcsr.into());
        csr_map.insert(CSR_DPC.into(), dpc.into());
        csr_map.insert(CSR_DSCRATCH0.into(), dscratch0.into());
        csr_map.insert(CSR_DSCRATCH1.into(), dscratch1.into());

        Self {
            config,
            csr_map,
            xstatus: xstatus_share,
            xip: xip_share,
            xie: xie_share,
            mcause: mcause_share,
            scause: scause_share,
            medeleg: medeleg_share,
            mideleg: mideleg_share,
            mepc: mepc_share,
            sepc: sepc_share,
            mtval: mtval_share,
            stval: stval_share,
            satp: satp_share,
            cycle: cycle_share,
            instret: instret_share,
            cur_priv: PrivilegeLevels::Machine,
            mtvec: mtvec_share,
            stvec: stvec_share,
            dcsr: dcsr_share,
            dpc: dpc_share,
        }
    }

    pub fn add_mtime(&mut self, mtime: RcCell<u64>) {
        let time = Counter::new(mtime);
        self.csr_map.insert(CSR_TIME.into(), time.into());
    }

    pub fn read(&mut self, addr: u64, privi: PrivilegeLevels) -> Result<u64, TrapType> {
        assert!(addr < 4096); // The size of a CSR is 4KB
        self.cur_priv = privi; // Update the current privilege level

        // Get the CSR with address addr from the CSR map. If it does not exist, return an illegal instruction trap.
        let csr = match self.csr_map.get(&addr) {
            Some(csr) => csr,
            None => return Err(TrapType::IllegalInstruction(0)),
        };

        // Check the permission of the CSR. If it is not allowed to be read, return an illegal instruction trap.
        if csr
            .check_permission(addr, privi, AccessType::Load(0))
            .is_err()
        {
            return Err(TrapType::IllegalInstruction(0));
        }

        // Return the value of the CSR.
        Ok(csr.read())
    }

    pub fn write(&mut self, addr: u64, data: u64, privi: PrivilegeLevels) -> Result<(), TrapType> {
        assert!(addr < 4096); // The size of a CSR is 4KB
        self.cur_priv = privi; // Update the current privilege level

        // Get the CSR with address addr from the CSR map. If it does not exist, return an illegal instruction trap.
        let csr = match self.csr_map.get_mut(&addr) {
            Some(csr) => csr,
            None => return Err(TrapType::IllegalInstruction(0)),
        };

        // Check the permission of the CSR. If it is not allowed, return an illegal instruction trap.
        if csr
            .check_permission(addr, privi, AccessType::Store(0))
            .is_err()
        {
            return Err(TrapType::IllegalInstruction(0));
        }

        // Return the value of the CSR.
        csr.write(data);
        Ok(())
    }

    pub fn write_raw(&mut self, addr: u64, data: u64) {
        assert!(addr < 4096); // The size of a CSR is 4KB

        // Get the CSR with address addr from the CSR map. If it does not exist, return
        let csr = match self.csr_map.get_mut(&addr) {
            Some(csr) => csr,
            None => return,
        };

        csr.write(data);
    }

    pub fn read_raw(&mut self, addr: u64) -> u64 {
        assert!(addr < 4096); // The size of a CSR is 4KB

        // Get the CSR with address addr from the CSR map. If it does not exist, return 0
        let csr = match self.csr_map.get(&addr) {
            Some(csr) => csr,
            None => return 0,
        };
        csr.read()
    }
}
