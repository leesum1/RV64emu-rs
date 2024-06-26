use alloc::boxed::Box;
use bitfield_struct::bitfield;
use enum_dispatch::enum_dispatch;

use crate::{
    rv64core::inst::inst_base::{AccessType, PrivilegeLevels},
    rv64core::traptype::TrapType,
    tools::RcCell,
};

use super::inst::inst_base::RVerr;

#[enum_dispatch]
pub enum CsrEnum {
    ReadOnlyCSR,
    CommonCSR,
    Misa,
    Xstatus,
    Xtvec,
    Xie,
    Xip,
    Xcause,
    Medeleg,
    // Mideleg,
    Mcounteren,
    Menvcfg,
    Mseccfg,
    PMPcfg,
    PMPaddr,
    Satp,
    Counter,
    Dcsr,
}

#[enum_dispatch(CsrEnum)]
pub trait Csr {
    fn read(&self) -> u64 {
        self.read_raw()
    }
    fn write(&mut self, _data: u64) {}
    fn read_raw(&self) -> u64;

    fn check_permission(
        &self,
        addr: u64,
        privi: PrivilegeLevels,
        access_type: AccessType,
    ) -> Result<(), RVerr> {
        assert!(addr < 4096);
        let csr_addr = CsrAddr::from(addr as u16);
        match csr_addr.check_privilege(privi, access_type) {
            true => Ok(()),
            false => Err(RVerr::CsrNotPermit),
        }
    }
}
fn write_with_mask(old: u64, data: u64, mask: u64) -> u64 {
    (old & !mask) | (data & mask)
}

pub struct ReadOnlyCSR(pub u64);
impl Csr for ReadOnlyCSR {
    fn read_raw(&self) -> u64 {
        self.0
    }
}

pub struct CommonCSR {
    inner: RcCell<u64>,
}

impl CommonCSR {
    pub fn new(share: RcCell<u64>) -> Self {
        Self { inner: share }
    }
    pub fn new_noshare(data: u64) -> Self {
        Self {
            inner: RcCell::new(data.into()),
        }
    }
}

impl Csr for CommonCSR {
    fn write(&mut self, data: u64) {
        self.inner.set(data);
    }
    fn read_raw(&self) -> u64 {
        self.inner.get()
    }
}

#[bitfield(u16)]
pub struct CsrAddr {
    #[bits(8)]
    addr: u8,
    #[bits(2)]
    privilege: u8,
    #[bits(2)]
    read_write: u8,
    #[bits(4)]
    _pad: u8,
}

// CSR address (csr[11:8]) are used to encode the read and
// write accessibility of the CSRs according to privilege level as shown in Table 2.1. The top two bits
// (csr[11:10]) indicate whether the register is read/write (00, 01, or 10) or read-only (11). The next
// two bits (csr[9:8]) encode the lowest privilege level that can access the CSR.

// Attempts to access a non-existent CSR raise an illegal instruction exception. Attempts to access a
// CSR without appropriate privilege level or to write a read-only register also raise illegal instruction
// exceptions. A read/write register might also contain some bits that are read-only, in which case
// writes to the read-only bits are ignored.
impl CsrAddr {
    pub fn check_privilege(&self, privi: PrivilegeLevels, access_type: AccessType) -> bool {
        let has_privilege = (privi as u8) >= self.privilege();
        // warn!("privi:{:?},{}", privi, has_privilege);
        match access_type {
            AccessType::Store(_) => self.not_read_only() && has_privilege,
            _ => has_privilege,
        }
    }

    fn not_read_only(&self) -> bool {
        let read_only = self.read_write() == 0b11;
        !read_only // not read only
    }
}

#[bitfield(u64)]
pub struct Misa {
    pub a: bool,
    pub b: bool,
    pub c: bool,
    pub d: bool,
    pub e: bool,
    pub f: bool,
    pub g: bool,
    pub h: bool,
    pub i: bool,
    pub j: bool,
    pub k: bool,
    pub l: bool,
    pub m: bool,
    pub n: bool,
    pub o: bool,
    pub p: bool,
    pub q: bool,
    pub r: bool,
    pub s: bool,
    pub t: bool,
    pub u: bool,
    pub v: bool,
    pub w: bool,
    pub x: bool,
    pub y: bool,
    pub z: bool,
    #[bits(36)]
    _pad: u64,
    #[bits(2)]
    pub mxl: u8,
}
// read only
impl Csr for Misa {
    fn read_raw(&self) -> u64 {
        self.0
    }
}

#[bitfield(u64)]
pub struct XstatusIn {
    _wpri0: bool,
    pub sie: bool,
    _wpri1: bool,
    pub mie: bool,
    _wpri2: bool,
    pub spie: bool,
    pub ube: bool,
    pub mpie: bool,
    pub spp: bool,
    #[bits(2)]
    pub vs: u8,
    #[bits(2)]
    pub mpp: u8,
    #[bits(2)]
    pub fs: u8,
    #[bits(2)]
    pub xs: u8,
    pub mprv: bool,
    pub sum: bool,
    pub mxr: bool,
    pub tvm: bool,
    pub tw: bool,
    pub tsr: bool,
    #[bits(9)]
    _wpri3: u16,
    #[bits(2)]
    pub uxl: u8,
    #[bits(2)]
    pub sxl: u8,
    pub sbe: bool,
    pub mbe: bool,
    #[bits(25)]
    _wpri4: u32,
    pub sd: bool,
}

impl XstatusIn {
    // When a trap is taken, SPP is set to 0 if the trap originated from user mode, or 1 otherwise
    // 0: user mode 1: s mode
    pub fn get_spp_priv(&self) -> PrivilegeLevels {
        match self.spp() {
            true => PrivilegeLevels::Supervisor,
            false => PrivilegeLevels::User,
        }
    }
    pub fn get_mpp_priv(&self) -> PrivilegeLevels {
        // PrivilegeLevels::from_repr(self.mpp().into()).unwrap()
        match self.mpp() {
            0b00 => PrivilegeLevels::User,
            0b01 => PrivilegeLevels::Supervisor,
            // 0b10 => PrivilegeLevels::Hypervisor,
            0b11 => PrivilegeLevels::Machine,
            _ => panic!("invalid mpp value"),
        }
    }
    pub fn update_sd(&mut self) {
        let sd_val = self.fs() == 0b11 || self.xs() == 0b11 || self.vs() == 0b11;
        self.set_sd(sd_val);
    }
}

pub struct Xstatus {
    inner: RcCell<XstatusIn>,
    rmask: u64,
    wmask: u64,
}

impl Xstatus {
    pub fn new(share: RcCell<XstatusIn>, rmask: u64, wmask: u64) -> Self {
        Self {
            inner: share,
            rmask,
            wmask,
        }
    }
}

impl Csr for Xstatus {
    fn write(&mut self, data: u64) {
        let new_data = write_with_mask(self.inner.get().into(), data, self.wmask);
        let mut status = XstatusIn::from(new_data);
        status.update_sd();
        self.inner.set(status);
    }
    fn read(&self) -> u64 {
        let data = self.read_raw();
        data & self.rmask
    }
    fn read_raw(&self) -> u64 {
        let mut status = self.inner.get();
        status.update_sd();
        status.into()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TvecMode {
    Direct = 0,
    Vectored = 1,
    Reserved,
}

impl TvecMode {
    const fn into_bits(self) -> u64 {
        self as u64
    }
    const fn from_bits(v: u64) -> Self {
        match v {
            0 => Self::Direct,
            1 => Self::Vectored,
            _ => Self::Reserved,
        }
    }
}

#[bitfield(u64)]
pub struct XtvecIn {
    #[bits(2,default = TvecMode::Direct)]
    pub mode: TvecMode,
    #[bits(62)]
    pub base: u64,
}

impl XtvecIn {
    pub fn get_trap_pc(&self, trap: TrapType) -> u64 {
        let base = self.base() << 2;
        match self.mode() {
            TvecMode::Vectored if trap.is_interupt() => base + 4 * trap.get_irq_num(),
            TvecMode::Direct | TvecMode::Vectored => base,
            TvecMode::Reserved => panic!("invalid tvec mode:{:?}", self.mode()),
        }
    }

    pub fn get_write_mask(val: u64) -> u64 {
        let tvec = XtvecIn::from(val);
        if tvec.mode() == TvecMode::Reserved {
            0xFFFF_FFFF_FFFF_FFFF
        } else {
            0xFFFF_FFFF_FFFF_FFFC
        }
    }
}

pub struct Xtvec {
    inner: RcCell<XtvecIn>,
}

impl Xtvec {
    pub fn new(share: RcCell<XtvecIn>) -> Self {
        Self { inner: share }
    }
}

impl Csr for Xtvec {
    fn write(&mut self, data: u64) {
        self.inner.set(XtvecIn::from(data));
    }
    fn read_raw(&self) -> u64 {
        self.inner.get().into()
    }
}

#[bitfield(u64)]
pub struct XieIn {
    _pad0: bool,
    pub ssie: bool,
    _pad1: bool,
    pub msie: bool,
    _pad2: bool,
    pub stie: bool,
    _pad3: bool,
    pub mtie: bool,
    _pad4: bool,
    pub seie: bool,
    _pad5: bool,
    pub meie: bool,
    #[bits(52)]
    _pad6: u64,
}

pub struct Xie {
    inner: RcCell<XieIn>,
    mask: u64,
}

impl Xie {
    pub fn new(share: RcCell<XieIn>, mask: u64) -> Self {
        Self { inner: share, mask }
    }
}

impl Csr for Xie {
    fn write(&mut self, data: u64) {
        let mask = self.mask;
        let mut inner = self.inner.get();
        inner.0 = write_with_mask(inner.0, data, mask);
        self.inner.set(inner);
    }
    fn read_raw(&self) -> u64 {
        let inner = self.inner.get();
        inner.0 & self.mask
    }
}

#[bitfield(u64)]
pub struct XipIn {
    _pad0: bool,
    pub ssip: bool,
    _pad1: bool,
    pub msip: bool,
    _pad2: bool,
    pub stip: bool,
    _pad3: bool,
    pub mtip: bool,
    _pad4: bool,
    pub seip: bool,
    _pad5: bool,
    pub meip: bool,
    #[bits(52)]
    _pad6: u64,
}
// standard interrupt priority is MEI, MSI, MTI, SEI, SSI, STI
impl XipIn {
    pub fn get_priority_interupt(&self) -> TrapType {
        if self.meip() {
            return TrapType::MachineExternalInterrupt;
        } else if self.msip() {
            return TrapType::MachineSoftwareInterrupt;
        } else if self.mtip() {
            return TrapType::MachineTimerInterrupt;
        } else if self.seip() {
            return TrapType::SupervisorExternalInterrupt;
        } else if self.ssip() {
            return TrapType::SupervisorSoftwareInterrupt;
        } else if self.stip() {
            return TrapType::SupervisorTimerInterrupt;
        }
        panic!("no interupt:{self:?}");
    }
    pub fn set_irq(&mut self, irq_num: usize) {
        match irq_num {
            1 => self.set_ssip(true),
            3 => self.set_msip(true),
            5 => self.set_stip(true),
            7 => self.set_mtip(true),
            9 => self.set_seip(true),
            11 => self.set_meip(true),
            _ => panic!("invalid irq num:{}", irq_num),
        }
    }
}

pub struct Xip {
    inner: RcCell<XipIn>,
    mask: u64,
}

impl Xip {
    pub fn new(share: RcCell<XipIn>, mask: u64) -> Self {
        Self { inner: share, mask }
    }
}
impl Csr for Xip {
    fn read_raw(&self) -> u64 {
        self.inner.get().0 & self.mask
    }
    fn write(&mut self, data: u64) {
        let mask = self.mask;
        let mut inner = self.inner.get();
        inner.0 = write_with_mask(inner.0, data, mask);
        self.inner.set(inner);
    }
}

#[bitfield(u64)]
pub struct XcauseIn {
    #[bits(63)]
    pub exception_code: u64,
    pub interrupt: bool,
}

pub struct Xcause {
    inner: RcCell<XcauseIn>,
}

impl Xcause {
    pub fn new(share: RcCell<XcauseIn>) -> Self {
        Self { inner: share }
    }
}

impl Csr for Xcause {
    fn write(&mut self, data: u64) {
        self.inner.set(XcauseIn::from(data));
    }
    fn read_raw(&self) -> u64 {
        self.inner.get().0
    }
}

pub type Mideleg = Xip;
pub type MidelegIn = XipIn;

#[bitfield(u64)]
pub struct MedelegIn {
    pub inst_addr_misalign: bool,
    pub inst_access_fault: bool,
    pub illegal_inst: bool,
    pub breakpoint: bool,
    pub load_addr_misalign: bool,
    pub load_access_fault: bool,
    pub store_addr_misalign: bool,
    pub store_access_fault: bool,
    pub ecall_from_u: bool,
    pub ecall_from_s: bool,
    _reserved0: bool,
    pub ecall_from_m: bool,
    pub inst_page_fault: bool,
    pub load_page_fault: bool,
    _reserved1: bool,
    pub store_page_fault: bool,
    #[bits(48)]
    pub _reserved2: u64,
}

pub struct Medeleg {
    inner: RcCell<MedelegIn>,
}

impl Medeleg {
    pub fn new(share: RcCell<MedelegIn>) -> Self {
        Self { inner: share }
    }
}

impl Csr for Medeleg {
    fn write(&mut self, data: u64) {
        self.inner.set(MedelegIn::from(data));
    }
    fn read_raw(&self) -> u64 {
        self.inner.get().0
    }
}

#[bitfield(u64)]
pub struct Mcounteren {
    pub cy: bool,
    pub tm: bool,
    pub ir: bool,
    #[bits(29)]
    hmp3_31: u32,
    _pad: u32,
}

impl Csr for Mcounteren {
    fn read_raw(&self) -> u64 {
        self.0
    }
}

impl Mcounteren {
    pub fn hmp(&self, idx: usize) -> bool {
        assert!(idx >= 3);
        let idx_offset = idx - 3;
        ((self.hmp3_31() >> idx_offset) & 0b1) == 1
    }
    // todo! set val true or false
    pub fn set_hmp(&mut self, idx: usize) {
        assert!(idx >= 3);
        let idx_offset = idx - 3;
        let pre_hmp_n = self.hmp3_31();
        let next_hmp_n = pre_hmp_n | (1 << idx_offset);
        self.set_hmp3_31(next_hmp_n);
    }
}

#[bitfield(u64)]
pub struct Menvcfg {
    pub fiom: bool,
    #[bits(3)]
    _wpri0: u8,
    #[bits(2)]
    pub cbie: u8,
    pub cbcfe: bool,
    pub cbze: bool,
    #[bits(54)]
    _wpri1: u64,
    pub pbmte: bool,
    pub stce: bool,
}

impl Csr for Menvcfg {
    fn read_raw(&self) -> u64 {
        self.0
    }
}

#[bitfield(u64)]
pub struct Mseccfg {
    pub mml: bool,
    pub mmwp: bool,
    pub rlb: bool,
    #[bits(5)]
    _wpri0: u8,
    pub useed: bool,
    pub sseed: bool,
    #[bits(54)]
    _wpri0: u64,
}
impl Csr for Mseccfg {
    fn read_raw(&self) -> u64 {
        self.0
    }
}

#[bitfield(u8)]
pub struct PMPcfgIn {
    pub r: bool,
    pub w: bool,
    pub x: bool,
    #[bits(2)]
    pub a: u8,
    #[bits(2)]
    _pad: u8,
    pub l: bool,
}

impl PMPcfgIn {
    const fn into_bits(self) -> u64 {
        self.0 as u64
    }
    const fn from_bits(v: u64) -> Self {
        let mut new = Self::new();
        new.0 = v as u8;
        new
    }
}

// impl From<u64> for PMPcfgIn {
//     fn from(value: u64) -> Self {
//         PMPcfgIn::from(value as u8)
//     }
// }

// impl From<PMPcfgIn> for u64 {
//     fn from(val: PMPcfgIn) -> Self {
//         val.0 as u64
//     }
// }
pub type PMPcfgShare = RcCell<Box<PMPcfg>>;
#[bitfield(u64)]
pub struct PMPcfg {
    #[bits(8)]
    pub pmp0cfg: PMPcfgIn,
    #[bits(8)]
    pub pmp1cfg: PMPcfgIn,
    #[bits(8)]
    pub pmp2cfg: PMPcfgIn,
    #[bits(8)]
    pub pmp3cfg: PMPcfgIn,
    #[bits(8)]
    pub pmp4cfg: PMPcfgIn,
    #[bits(8)]
    pub pmp5cfg: PMPcfgIn,
    #[bits(8)]
    pub pmp6cfg: PMPcfgIn,
    #[bits(8)]
    pub pmp7cfg: PMPcfgIn,
}

impl Csr for PMPcfg {
    fn read_raw(&self) -> u64 {
        self.0
    }
}

#[bitfield(u64)]
pub struct PMPaddr {
    #[bits(54)]
    pub address_55_2: u64,
    #[bits(10)]
    _pad: u32,
}

impl Csr for PMPaddr {
    fn read_raw(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StapMode {
    Bare = 0,
    Sv39 = 8,
    Sv48 = 9,
    Sv57 = 10,
    Sv64 = 11,
}

impl StapMode {
    pub fn get_levels(&self) -> usize {
        match self {
            StapMode::Bare => 0,
            StapMode::Sv39 => 3,
            StapMode::Sv48 => 4,
            StapMode::Sv57 => 5,
            StapMode::Sv64 => 6,
        }
    }

    pub fn get_ptesize(&self) -> usize {
        match self {
            StapMode::Bare => 0,
            StapMode::Sv39 => 8,
            StapMode::Sv48 => 8,
            StapMode::Sv57 => 8,
            StapMode::Sv64 => 8,
        }
    }

    const fn into_bits(self) -> u64 {
        self as u64
    }
    const fn from_bits(v: u64) -> Self {
        match v {
            0 => StapMode::Bare,
            8 => StapMode::Sv39,
            9 => StapMode::Sv48,
            10 => StapMode::Sv57,
            11 => StapMode::Sv64,
            _ => panic!("invalid satp mode"),
        }
    }
}

// impl From<u64> for StapMode {
//     fn from(value: u64) -> Self {
//         match value {
//             0 => StapMode::Bare,
//             8 => StapMode::Sv39,
//             9 => StapMode::Sv48,
//             10 => StapMode::Sv57,
//             11 => StapMode::Sv64,
//             val => panic!("StapMode:{val}"),
//         }
//     }
// }

// impl From<StapMode> for u64 {
//     fn from(val: StapMode) -> Self {
//         val as u64
//     }
// }

#[bitfield(u64)]
pub struct SatpIn {
    #[bits(44)]
    pub ppn: u64,
    #[bits(16)]
    pub asid: u64,
    #[bits(4)]
    pub mode: StapMode,
}

pub struct Satp {
    inner: RcCell<SatpIn>,
    xstatus: RcCell<XstatusIn>,
    max_satp_mode: StapMode,
}

impl Satp {
    pub fn new(
        share: RcCell<SatpIn>,
        xstatus_share: RcCell<XstatusIn>,
        max_mode: StapMode,
    ) -> Self {
        Satp {
            inner: share,
            xstatus: xstatus_share,
            max_satp_mode: max_mode,
        }
    }
}

impl Satp {
    fn unsupport_mod(&self, new_mode: StapMode) -> bool {
        new_mode as usize > self.max_satp_mode as usize
    }
}

impl Csr for Satp {
    fn write(&mut self, data: u64) {
        let new_val = SatpIn::from(data);

        let mut stap = self.inner.get();
        if !self.unsupport_mod(new_val.mode()) {
            stap.set_mode(new_val.mode());
        }
        stap.set_asid(new_val.asid());
        stap.set_ppn(new_val.ppn());

        self.inner.set(stap);
    }
    fn read_raw(&self) -> u64 {
        self.inner.get().into()
    }

    fn check_permission(
        &self,
        _addr: u64,
        privi: PrivilegeLevels,
        _access_type: AccessType,
    ) -> Result<(), RVerr> {
        let tvm = self.xstatus.get().tvm();

        let require_priv = if tvm {
            PrivilegeLevels::Machine
        } else {
            PrivilegeLevels::Supervisor
        };
        // warn!("satp:cur_priv:{:?},require_priv:{:?}", privi, require_priv);
        match require_priv.check_priv(privi) {
            true => Ok(()),
            false => Err(RVerr::CsrNotPermit),
        }
    }
}

pub struct Counter {
    inner: RcCell<u64>,
}

impl Counter {
    pub fn new(share: RcCell<u64>) -> Self {
        Counter { inner: share }
    }
}

impl Csr for Counter {
    fn read_raw(&self) -> u64 {
        self.inner.get()
    }
}

#[bitfield(u32)]
pub struct DcsrIn {
    #[bits(2)]
    pub prv: u8,
    pub step: bool,
    pub nmip: bool,
    pub mprven: bool,
    pub v: bool,
    #[bits(3)]
    pub cause: u8,
    pub stoptime: bool,
    pub stopcount: bool,
    pub stepie: bool,
    pub ebreaku: bool,
    pub ebreaks: bool,
    pub zero0: bool,
    pub ebreakm: bool,
    pub ebreakvu: bool,
    pub ebreakvs: bool,
    #[bits(10)]
    pub zero1: u16,
    #[bits(4)]
    pub debugver: u8,
}

pub struct Dcsr {
    inner: RcCell<DcsrIn>,
}

impl Dcsr {
    pub fn new(share: RcCell<DcsrIn>) -> Self {
        Dcsr { inner: share }
    }
}

impl Csr for Dcsr {
    fn read_raw(&self) -> u64 {
        u32::from(self.inner.get()) as u64
    }

    fn write(&mut self, _data: u64) {
        let new_in = DcsrIn::from(_data as u32);
        let mut old_val = self.inner.get();

        // This bit is hardwired to 0 if the hart does not
        // support virtualization mode.
        old_val.set_ebreakvs(false);
        old_val.set_ebreakvu(false);

        old_val.set_ebreakm(new_in.ebreakm());
        old_val.set_ebreaks(new_in.ebreaks());
        old_val.set_ebreaku(new_in.ebreaku());
        old_val.set_step(new_in.step());

        // PrivilegeLevels::from_repr(new_in.prv()).unwrap();

        if let Some(new_prv) = PrivilegeLevels::from_usize(new_in.prv().into()) {
            old_val.set_prv(new_prv as u8);
        }

        old_val.set_stepie(false); // hard code to zero
        old_val.set_stopcount(false); // hard code to zero
        old_val.set_stoptime(false); // hard code to zero
        old_val.set_v(false); // hard code to zero
        old_val.set_mprven(true); // hard code to one (1 (enabled): MPRV in mstatus takes effect in Debug Mode.)

        self.inner.set(old_val);
    }
}
