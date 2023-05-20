use bitfield_struct::bitfield;

use super::vm_info::{PAops, PTEops, VAops};

//  Sv57 virtual address.
#[bitfield(u64)]
pub struct Sv57VA {
    #[bits(12)]
    pub offset: u64,
    #[bits(9)]
    pub ppn0: u64,
    #[bits(9)]
    pub ppn1: u64,
    #[bits(9)]
    pub ppn2: u64,
    #[bits(9)]
    pub ppn3: u64,
    #[bits(9)]
    pub ppn4: u64,
    #[bits(7)]
    _pad: u64,
}

impl VAops for Sv57VA {
    fn get_ppn_by_idx(&self, idx: u8) -> u64 {
        match idx {
            0 => self.ppn0(),
            1 => self.ppn1(),
            2 => self.ppn2(),
            3 => self.ppn3(),
            4 => self.ppn4(),
            _ => panic!("Sv57Va ppn idx err:{idx}"),
        }
    }

    fn offset(&self) -> usize {
        self.offset() as usize
    }

    fn set_offset(&mut self, val: usize) {
        self.set_offset(val as u64);
    }
    fn raw(&self) -> u64 {
        self.0
    }
}

#[bitfield(u64)]
pub struct Sv57PA {
    #[bits(12)]
    pub offset: u64,
    #[bits(9)]
    pub ppn0: u64,
    #[bits(9)]
    pub ppn1: u64,
    #[bits(9)]
    pub ppn2: u64,
    #[bits(9)]
    pub ppn3: u64,
    #[bits(8)]
    pub ppn4: u64,
    #[bits(8)]
    _pad: u64,
}

impl PAops for Sv57PA {
    fn set_ppn_by_idx(&mut self, val: u64, idx: u8) {
        match idx {
            0 => self.set_ppn0(val),
            1 => self.set_ppn1(val),
            2 => self.set_ppn2(val),
            3 => self.set_ppn3(val),
            4 => self.set_ppn4(val),
            _ => panic!("Sv57Pa ppn idx err:{idx}"),
        }
    }
    fn offset(&self) -> usize {
        self.offset() as usize
    }

    fn set_offset(&mut self, val: usize) {
        self.set_offset(val as u64);
    }
    fn raw(&self) -> u64 {
        self.0
    }
}

#[bitfield(u64)]
pub struct Sv57PTE {
    pub v: bool,
    pub r: bool,
    pub w: bool,
    pub x: bool,
    pub u: bool,
    pub g: bool,
    pub a: bool,
    pub d: bool,
    #[bits(2)]
    pub rsw: u8,
    #[bits(9)]
    pub ppn0: u64,
    #[bits(9)]
    pub ppn1: u64,
    #[bits(9)]
    pub ppn2: u64,
    #[bits(9)]
    pub ppn3: u64,
    #[bits(8)]
    pub ppn4: u64,
    #[bits(7)]
    pub reserved: u64,
    #[bits(2)]
    pub pbmt: u8,
    pub n: bool,
}

impl PTEops for Sv57PTE {
    fn raw(&self) -> u64 {
        self.0
    }
    fn get_ppn_by_idx(&self, idx: u8) -> u64 {
        match idx {
            0 => self.ppn0(),
            1 => self.ppn1(),
            2 => self.ppn2(),
            3 => self.ppn3(),
            4 => self.ppn4(),
            _ => panic!("Sv57PTE ppn idx err:{idx}"),
        }
    }

    fn v(&self) -> bool {
        self.v()
    }

    fn r(&self) -> bool {
        self.r()
    }

    fn w(&self) -> bool {
        self.w()
    }

    fn x(&self) -> bool {
        self.x()
    }

    fn u(&self) -> bool {
        self.u()
    }

    fn g(&self) -> bool {
        self.g()
    }

    fn a(&self) -> bool {
        self.a()
    }

    fn d(&self) -> bool {
        self.d()
    }

    fn rsw(&self) -> u8 {
        self.rsw()
    }

    fn pbmt(&self) -> u8 {
        self.pbmt()
    }

    fn n(&self) -> bool {
        self.n()
    }
}
