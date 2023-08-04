use super::sv39::{Sv39PA, Sv39PTE, Sv39VA};
use super::sv48::{Sv48PA, Sv48PTE, Sv48VA};
use super::sv57::{Sv57PA, Sv57PTE, Sv57VA};
use enum_dispatch::enum_dispatch;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PageSize {
    P4K,
    P2M,
    P1G,
    P512G,
    P256T,
    InValid,
}
impl PageSize {
    pub const fn from_i(val: usize) -> Self {
        match val {
            0 => PageSize::P4K,
            1 => PageSize::P2M,
            2 => PageSize::P1G,
            3 => PageSize::P512G,
            4 => PageSize::P256T,
            _ => panic!("Invalid page size"),
        }
    }
    pub const fn get_mask(&self) -> u64 {
        match self {
            PageSize::P4K => zero_mask(12),
            PageSize::P2M => zero_mask(21),
            PageSize::P1G => zero_mask(30),

            _ => panic!("Invalid page size"),
        }
    }
}

#[derive(Copy, Clone)]
pub struct TLBEntry {
    pub pte: PTEenume,
    pub page_size: PageSize,
    pub asid: u16,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct TLBKey {
    pub va: u64,
    pub asid: u16,
}

impl core::fmt::Debug for TLBEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "TLBEntry: ppn_all:{:x}, page_size:{:?}, asid:{:?},Global:{:?}",
            self.pte.ppn_all(),
            self.page_size,
            self.asid,
            self.pte.g()
        )
    }
}

// num: 0~64, the zero number in lsbs
const fn zero_mask(num: usize) -> u64 {
    if num == 64 {
        return 0;
    }
    u64::MAX << num
}

#[test]
fn zero_mast_test() {
    assert_eq!(zero_mask(0), u64::MAX);
    assert_eq!(zero_mask(1), u64::MAX << 1);
    assert_eq!(zero_mask(2), u64::MAX << 2);
    assert_eq!(zero_mask(64), 0);
}

impl TLBEntry {
    pub fn new(pte: PTEenume, page_size: PageSize, asid: u16) -> Self {
        Self {
            pte,
            page_size,
            asid,
        }
    }

    pub fn get_pa(&self, va: &VAenume) -> u64 {
        // debug!("pagesize:{:?}", self.page_size);
        match self.page_size {
            PageSize::P4K => (self.pte.ppn_all() << 12) | va.offset() as u64,
            PageSize::P2M => {
                ((self.pte.ppn_all() & zero_mask(9)) << 12)
                    | va.get_ppn_by_idx(0) << 12
                    | va.offset() as u64
            }
            PageSize::P1G => {
                ((self.pte.ppn_all() & zero_mask(18)) << 12)
                    | va.get_ppn_by_idx(1) << (12 + 9)
                    | va.get_ppn_by_idx(0) << 12
                    | va.offset() as u64
            }
            PageSize::P512G => {
                ((self.pte.ppn_all() & zero_mask(27)) << 12)
                    | va.get_ppn_by_idx(2) << (12 + 9 + 9)
                    | va.get_ppn_by_idx(1) << (12 + 9)
                    | va.get_ppn_by_idx(0) << 12
                    | va.offset() as u64
            }
            PageSize::P256T => {
                ((self.pte.ppn_all() & zero_mask(36)) << 12)
                    | va.get_ppn_by_idx(3) << (12 + 9 + 9 + 9)
                    | va.get_ppn_by_idx(2) << (12 + 9 + 9)
                    | va.get_ppn_by_idx(1) << (12 + 9)
                    | va.get_ppn_by_idx(0) << 12
                    | va.offset() as u64
            }
            PageSize::InValid => panic!("Invalid page size"),
        }
    }
}

#[enum_dispatch(PTEenume)]
pub trait PTEops {
    fn raw(&self) -> u64;
    fn get_ppn_by_idx(&self, idx: u8) -> u64;
    fn ppn_all(&self) -> u64 {
        (self.raw() >> 10) & 0xfff_ffff_ffff
    }
    fn point_next_level(&self) -> bool {
        // 0 0 0
        !(self.x() | self.w() | self.r())
    }
    //
    fn v(&self) -> bool;
    fn r(&self) -> bool;
    fn w(&self) -> bool;
    fn x(&self) -> bool;
    fn u(&self) -> bool;
    fn g(&self) -> bool;
    fn a(&self) -> bool;
    fn d(&self) -> bool;
    fn rsw(&self) -> u8;
    fn pbmt(&self) -> u8;
    fn n(&self) -> bool;
}

#[enum_dispatch(PAenume)]
pub trait PAops {
    fn set_ppn_by_idx(&mut self, val: u64, idx: u8);
    fn offset(&self) -> usize;
    fn set_offset(&mut self, val: usize);
    fn raw(&self) -> u64;
}

#[enum_dispatch(VAenume)]
pub trait VAops {
    fn get_ppn_by_idx(&self, idx: u8) -> u64;
    fn offset(&self) -> usize;
    fn set_offset(&mut self, val: usize);
    fn raw(&self) -> u64;
}

#[enum_dispatch]
#[derive(Copy, Clone)]
pub enum PTEenume {
    Sv39PTE,
    Sv48PTE,
    Sv57PTE,
}

#[enum_dispatch]
pub enum PAenume {
    Sv39PA,
    Sv48PA,
    Sv57PA,
}

#[enum_dispatch]
pub enum VAenume {
    Sv39VA,
    Sv48VA,
    Sv57VA,
}
