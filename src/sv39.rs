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

#[bitfield(u64)]
pub struct Sv39Va {
    #[bits(12)]
    pub offset: u64,
    #[bits(9)]
    pub ppn0: u64,
    #[bits(9)]
    pub ppn1: u64,
    #[bits(9)]
    pub ppn2: u64,
    #[bits(25)]
    _pad: u64,
}

impl Sv39Va {
    pub fn get_ppn_by_idx(&mut self, idx: u8) -> u64 {
        match idx {
            0 => self.ppn0(),
            1 => self.ppn1(),
            2 => self.ppn2(),
            _ => panic!("idx err:{idx}"),
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
#[bitfield(u64)]
pub struct Sv39PTE {
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
    #[bits(26)]
    pub ppn2: u64,
    #[bits(7)]
    pub reserved: u64,
    #[bits(2)]
    pub pbmt: u8,
    pub n: bool,
}

impl Sv39PTE {
    pub fn ppn_all(&self) -> u64 {
        (self.0 >> 10) & 0xfff_ffff_ffff
    }

    pub fn get_ppn_by_idx(&mut self, idx: u8) -> u64 {
        match idx {
            0 => self.ppn0(),
            1 => self.ppn1(),
            2 => self.ppn2(),
            _ => panic!("idx err:{idx}"),
        }
    }
}

