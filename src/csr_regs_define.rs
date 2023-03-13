use std::u8;

use bitfield_struct::bitfield;
use strum_macros::FromRepr;

use crate::{
    inst::inst_base::{AccessType, PrivilegeLevels},
    traptype::TrapType,
};

#[bitfield(u64)]
pub struct CsrAddr {
    #[bits(8)]
    addr: u8,
    #[bits(2)]
    privilege: u8,
    #[bits(2)]
    read_write: u8,
    #[bits(52)]
    _pad: u64,
}
// Attempts to access a non-existent CSR raise an illegal instruction exception. Attempts to access a
// CSR without appropriate privilege level or to write a read-only register also raise illegal instruction
// exceptions. A read/write register might also contain some bits that are read-only, in which case
// writes to the read-only bits are ignored.
impl CsrAddr {
    pub fn check_privilege(&self, privi: PrivilegeLevels, access_type: AccessType) -> bool {
        let privi_check = (privi as u8) >= self.privilege();
        // println!("privi:{:?},{}", privi, privi_check);
        match access_type {
            AccessType::Store => self.not_read_only() && privi_check,
            _ => privi_check,
        }
    }

    fn not_read_only(&self) -> bool {
        let read_only = self.read_write() == 0b11;
        // println!("readonly:{}", read_only);
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

pub type Sstatus = Mstatus;
#[bitfield(u64)]
pub struct Mstatus {
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

impl Mstatus {
    // When a trap is taken, SPP is set to 0 if the trap originated from user mode, or 1 otherwise
    // 0: user mode 1: s mode
    pub fn get_spp_priv(&self) -> PrivilegeLevels {
        match self.spp() {
            true => PrivilegeLevels::Supervisor,
            false => PrivilegeLevels::User,
        }
    }
    pub fn get_mpp_priv(&self) -> PrivilegeLevels {
        PrivilegeLevels::from_repr(self.mpp().into()).unwrap()
    }
}

pub type Stvec = Mtvec;
#[bitfield(u64)]
pub struct Mtvec {
    #[bits(2)]
    pub mode: u8,
    #[bits(62)]
    pub base: u64,
}

#[repr(u8)]
#[derive(FromRepr)]
pub enum TvecMode {
    Direct = 0,
    Vectored = 1,
}

impl Mtvec {
    pub fn get_trap_pc(&self, trap: TrapType) -> u64 {
        let mode = TvecMode::from_repr(self.mode()).expect("Mtvec mode err");
        let base = self.base() << 2;
        match mode {
            TvecMode::Vectored if trap.is_interupt() => base + 4 * trap.get_irq_num(),
            TvecMode::Direct | TvecMode::Vectored => base,
        }
    }
}
pub type MieMip = Mie;
#[bitfield(u64)]
pub struct Mie {
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

pub type Sip = Mip;
pub type Sie = Mie;

#[bitfield(u64)]
pub struct Mip {
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
impl Mip {
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
}
pub type Scause = Mcause;
#[bitfield(u64)]
pub struct Mcause {
    #[bits(63)]
    pub exception_code: u64,
    pub interrupt: bool,
}

pub type Mideleg = Mip;

#[bitfield(u64)]
pub struct Medeleg {
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

pub type Mcountinhibit = Mcounteren;
#[bitfield(u64)]
pub struct Mcounteren {
    pub cy: bool,
    pub tm: bool,
    pub ir: bool,
    #[bits(29)]
    hmp3_31: u32,
    _pad: u32,
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

impl From<u64> for PMPcfgIn {
    fn from(value: u64) -> Self {
        PMPcfgIn::from(value as u8)
    }
}

impl From<PMPcfgIn> for u64 {
    fn from(val: PMPcfgIn) -> Self {
        val.0 as u64
    }
}

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

#[bitfield(u64)]
pub struct PMPaddr {
    #[bits(54)]
    pub address_55_2: u64,
    #[bits(10)]
    _pad: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StapMode {
    Bare = 0,
    Sv39 = 8,
    Sv48 = 9,
    Sv57 = 10,
    Sv64 = 11,
}

impl From<u64> for StapMode {
    fn from(value: u64) -> Self {
        match value {
            0 => StapMode::Bare,
            8 => StapMode::Sv39,
            9 => StapMode::Sv48,
            10 => StapMode::Sv57,
            11 => StapMode::Sv64,
            val => panic!("StapMode:{val}"),
        }
    }
}

impl From<StapMode> for u64 {
    fn from(val: StapMode) -> Self {
        val as u64
    }
}

#[bitfield(u64)]
pub struct Stap {
    #[bits(44)]
    pub ppn: u64,
    #[bits(16)]
    pub asid: u64,
    #[bits(4)]
    pub mode: StapMode,
}
