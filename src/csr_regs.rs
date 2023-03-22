use std::{cell::Cell, rc::Rc};

use hashbrown::HashMap;

use crate::{
    csr_regs_define::{
        CommonCSR, Counter, Csr, CsrAddr, CsrEnum, Medeleg, MedelegIn, Mideleg, MidelegIn,
        Misa, ReadOnlyCSR, Satp, SatpIn, Xcause, XcauseIn, Xie, XieIn, Xip, XipIn, Xstatus,
        XstatusIn, Xtvec, XtvecIn,
    },
    inst::inst_base::{
        AccessType, PrivilegeLevels, CSR_CYCLE, CSR_INSTRET, CSR_MARCHID, CSR_MCAUSE, CSR_MCYCLE,
        CSR_MEDELEG, CSR_MEPC, CSR_MHARTID, CSR_MIDELEG, CSR_MIE, CSR_MIMPID, CSR_MINSTRET,
        CSR_MIP, CSR_MISA, CSR_MSCRATCH, CSR_MSTATUS, CSR_MTVAL, CSR_MTVEC, CSR_MVENDORID,
        CSR_SATP, CSR_SCAUSE, CSR_SEPC, CSR_SIE, CSR_SIP, CSR_SSCRATCH, CSR_SSTATUS, CSR_STVAL,
        CSR_STVEC, CSR_TIME, MASK_ALL,
    },
    traptype::TrapType,
};
type CsrShare<T> = Rc<Cell<T>>;

pub struct CsrRegs {
    pub csr_map: HashMap<u64, CsrEnum>,
    pub cur_priv: PrivilegeLevels,
    pub xstatus: CsrShare<XstatusIn>,
    pub xip: CsrShare<XipIn>,
    pub xie: CsrShare<XieIn>,
    pub mtvec: CsrShare<XtvecIn>,
    pub stvec: CsrShare<XtvecIn>,
    pub mcause: CsrShare<XcauseIn>,
    pub scause: CsrShare<XcauseIn>,
    pub medeleg: CsrShare<MedelegIn>,
    pub mideleg: CsrShare<MidelegIn>,
    pub mepc: CsrShare<u64>,
    pub sepc: CsrShare<u64>,
    pub satp: CsrShare<SatpIn>,
    pub mtval: CsrShare<u64>,
    pub stval: CsrShare<u64>,
    pub time: CsrShare<u64>,
    pub cycle: CsrShare<u64>,
    pub instret: CsrShare<u64>,
}
unsafe impl Send for CsrRegs {}

impl CsrRegs {
    pub fn new() -> Self {
        let misa_val = Misa::new()
            .with_i(true)
            .with_m(true)
            .with_a(true)
            .with_s(true)
            .with_u(true)
            .with_mxl(2); // 64
        let mstatus_val = XstatusIn::new()
            .with_uxl(misa_val.mxl())
            .with_sxl(misa_val.mxl());

        // read only
        let misa = misa_val;
        let mhartid = ReadOnlyCSR(0);
        let marchid = ReadOnlyCSR(0);
        let mvendorid = ReadOnlyCSR(0);
        let mimpid = ReadOnlyCSR(0);
        // important csrs
        let status_share = Rc::new(Cell::new(mstatus_val));
        let mstatus = Xstatus::new(status_share.clone(), MASK_ALL);
        let sstatus = Xstatus::new(status_share.clone(), MASK_ALL);

        let xip_share = Rc::new(Cell::new(XipIn::new()));
        let mip = Xip::new(xip_share.clone(), MASK_ALL);
        let sip = Xip::new(xip_share.clone(), MASK_ALL);

        let xie_share = Rc::new(Cell::new(XieIn::new()));
        let mie = Xie::new(xie_share.clone(), MASK_ALL);
        let sie = Xie::new(xie_share.clone(), MASK_ALL);

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
        let satp = Satp::new(satp_share.clone());

        let cycle_share = Rc::new(Cell::new(0));
        let mcycle = Counter::new(cycle_share.clone());
        let cycle = Counter::new(cycle_share.clone());

        let time_share = Rc::new(Cell::new(0));
        let _mtime = Counter::new(time_share.clone());
        let time = Counter::new(time_share.clone());

        let instret_share = Rc::new(Cell::new(0));
        let minstret = Counter::new(instret_share.clone());
        let instret = Counter::new(instret_share.clone());

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
        csr_map.insert(CSR_TIME.into(), time.into());
        csr_map.insert(CSR_MINSTRET.into(), minstret.into());
        csr_map.insert(CSR_INSTRET.into(), instret.into());

        Self {
            csr_map,
            xstatus: status_share,
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
            time: time_share,
            cycle: cycle_share,
            instret: instret_share,
            cur_priv: PrivilegeLevels::Machine,
            mtvec: mtvec_share,
            stvec: stvec_share,
        }
    }

    // CSR address (csr[11:8]) are used to encode the read and
    // write accessibility of the CSRs according to privilege level as shown in Table 2.1. The top two bits
    // (csr[11:10]) indicate whether the register is read/write (00, 01, or 10) or read-only (11). The next
    // two bits (csr[9:8]) encode the lowest privilege level that can access the CSR.

    // Attempts to access a non-existent CSR raise an illegal instruction exception. Attempts to access a
    // CSR without appropriate privilege level or to write a read-only register also raise illegal instruction
    // exceptions. A read/write register might also contain some bits that are read-only, in which case
    // writes to the read-only bits are ignored.
    fn check_csr(&mut self, addr: u64, privi: PrivilegeLevels, access_type: AccessType) -> bool {
        let csr_addr = CsrAddr::from(addr);
        csr_addr.check_privilege(privi, access_type)
    }

    pub fn read(&mut self, addr: u64, privi: PrivilegeLevels) -> Result<u64, TrapType> {
        assert!(addr < 4096); // The size of a CSR is 4KB
        self.cur_priv = privi; // Update the current privilege level

        // Check if the CSR is accessible
        if !self.check_csr(addr, privi, AccessType::Load(0)) {
            return Err(TrapType::IllegalInstruction(addr));
        };

        match self.csr_map.get(&addr) {
            Some(csr) => Ok(csr.read()),
            None => {
                // panic!("csr:{addr:x}")
                Err(TrapType::IllegalInstruction(addr))
            }
        }
    }

    pub fn write(&mut self, addr: u64, data: u64, privi: PrivilegeLevels) -> Result<(), TrapType> {
        assert!(addr < 4096); // The size of a CSR is 4KB
        self.cur_priv = privi; // Update the current privilege level

        // Check if the CSR is accessible
        if !self.check_csr(addr, privi, AccessType::Store(0)) {
            return Err(TrapType::IllegalInstruction(addr));
        };

        match self.csr_map.get_mut(&addr) {
            Some(csr) => {
                csr.write(data);
                Ok(())
            }
            None => {
                // panic!("csr:{addr:x}")
                Err(TrapType::IllegalInstruction(addr))
            }
        }
    }
}


#[test]
fn csr_new1(){

    let mut csr_regs = CsrRegs::new();

    let x = csr_regs.read(0x0305, PrivilegeLevels::Machine);

    println!("{:?}", x);

    let y = csr_regs.write(0x0305, 0x1234, PrivilegeLevels::Machine);

    println!("{:?}", y);
}