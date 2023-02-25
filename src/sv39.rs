// 38         30 29            21 20          12 11               0
// +-------------+---------------+--------------+-----------------+
// |   VPN[2]    |    VPN[1]     |    VPN[0]    |    page offset  |
// +-------------+---------------+--------------+-----------------+
//        9               9             9               12
//                      Sv39 virtual address.

//  55                30 29            21 20          12 11               0
// +---------------------+---------------+--------------+-----------------+
// |       PPN[2]        |     PPN[1]    |    PPN[0]    |    page offset  |
// +---------------------+---------------+--------------+-----------------+
//          26                   9              9               12
//                     Sv39 physical address.

//   63  62    61 60          54 53     28 27     19 18    10  9    8  7   6   5   4   3   2   1   0
// +----+--------+--------------+---------+---------+---------+------+---+---+---+---+---+---+---+---+
// | N  | PBMT   |  Reserved    |  PPN[2] |  PPN[1] |  PPN[0] | RSW  | D | A | G | U | X | W | R | V |
// +----+--------+--------------+---------+---------+---------+------+---+---+---+---+---+---+---+---+
//    1     2          7             26        9         9       2     1   1   1   1   1   1   1   1
//                                Sv39 page table entry.

use bitfield_struct::bitfield;
// Sv39 virtual address
pub struct Sv39Va {
    val: u64,
}

impl Sv39Va {
    pub fn new(val: u64) -> Self {
        Sv39Va { val }
    }

    pub fn offset(&self) -> u64 {
        self.val & 0xfff
    }

    pub fn vpn(&self, level: u8) -> u64 {
        match level {
            0 => (self.val >> 12) & 0x1ff,
            1 => (self.val >> 21) & 0x1ff,
            2 => (self.val >> 30) & 0x1ff,
            _ => panic!(),
        }
    }
}

#[bitfield(u64)]
pub struct Sv39Pa {
    #[bits(12)]
    pub offset: u64,
    #[bits(9)]
    pub ppn0: u64,
    #[bits(9)]
    pub ppn1: u64,
    #[bits(26)]
    pub ppn2: u64,
    #[bits(8)]
    _pad: u64,
}
impl Sv39Pa {
    pub fn set_ppn_by_idx(&mut self, val: u64, idx: u8) {
        match idx {
            0 => self.set_ppn0(val),
            1 => self.set_ppn1(val),
            2 => self.set_ppn2(val),
            _ => panic!("idx err:{idx}"),
        }
    }
}

pub struct Sv39PTE {
    val: u64,
}

impl Sv39PTE {
    pub fn ppn(&self, level: u8) -> u64 {
        match level {
            0 => (self.val >> 10) & 0x1ff,
            1 => (self.val >> 19) & 0x1ff,
            2 => (self.val >> 28) & 0x3ffffff,
            _ => panic!(),
        }
    }

    pub fn ppn_all(&self) -> u64 {
        (self.val >> 10) & 0xfff_ffff_ffff
    }

    pub fn v(&self) -> bool {
        (self.val & 0x1) != 0
    }
    pub fn r(&self) -> bool {
        ((self.val >> 1) & 0x1) != 0
    }
    pub fn w(&self) -> bool {
        ((self.val >> 2) & 0x1) != 0
    }
    pub fn x(&self) -> bool {
        ((self.val >> 3) & 0x1) != 0
    }
    pub fn u(&self) -> bool {
        ((self.val >> 4) & 0x1) != 0
    }
    pub fn g(&self) -> bool {
        ((self.val >> 5) & 0x1) != 0
    }
    pub fn a(&self) -> bool {
        ((self.val >> 6) & 0x1) != 0
    }
    pub fn d(&self) -> bool {
        ((self.val >> 7) & 0x1) != 0
    }
    pub fn rsw(&self) -> u64 {
        (self.val >> 8) & 0x3
    }
    pub fn pbmt(&self) -> u64 {
        (self.val >> 61) & 0x3
    }
    pub fn n(&self) -> bool {
        ((self.val >> 63) & 0x1) != 0
    }
}

impl Sv39PTE {
    pub fn new(val: u64) -> Self {
        Sv39PTE { val }
    }

    pub fn offset(&self) -> u64 {
        self.val & 0xfff
    }

    pub fn vpn(&self, level: u8) -> u64 {
        match level {
            0 => (self.val >> 12) & 0x1ff,
            1 => (self.val >> 21) & 0x1ff,
            2 => (self.val >> 30) & 0x3ffffff,
            _ => panic!(),
        }
    }
}
