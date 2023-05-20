use super::sv39::{Sv39PA, Sv39PTE, Sv39VA};
use super::sv48::{Sv48PA, Sv48PTE, Sv48VA};
use super::sv57::{Sv57PA, Sv57PTE, Sv57VA};
use enum_dispatch::enum_dispatch;

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
