use alloc::vec::Vec;

use crate::{rv64core::csr_regs_define::XipIn, tools::CsrShare};

use super::device_trait::DeviceBase;

const MSIP_BASE: u64 = 0x0;
const MSIP_PER_HART: u64 = 0x4;
const MSIP_END: u64 = MTIMECMP_BASE - 1;

const MTIMECMP_BASE: u64 = 0x4000;
const MTIMECMP_PER_HART: u64 = 0x8;
const MTIMECMP_END: u64 = MTIME_BASE - 1;
const MTIME_BASE: u64 = 0xBFF8;
const MTIME_BASE_END: u64 = 0xBFF8 + 7;

pub struct DeviceClint {
    pub start: u64,
    pub len: u64,
    pub instance: Clint,
    pub name: &'static str,
}

// each hart has a memory maped mtimcmp
// xip is a shared resource with cpu core
struct ClintHart {
    mtimecmp: u64,
    xip: CsrShare<XipIn>,
}

impl ClintHart {
    pub fn new(xip_share: CsrShare<XipIn>) -> Self {
        ClintHart {
            mtimecmp: u64::MAX,
            xip: xip_share,
        }
    }
    pub fn msip_read(&self) -> u64 {
        let msip = self.xip.get().msip();
        msip as u64
    }
    pub fn msip_write(&mut self, data: u64) {
        let mut xip = self.xip.get();
        xip.set_msip(data == 1);
        self.xip.set(xip);
    }

    pub fn mtimecmp_read(&self) -> u64 {
        self.mtimecmp
    }

    pub fn mtimecmp_write(&mut self, data: u64) {
        self.mtimecmp = data;
    }
    pub fn mtimecmph_read(&self) -> u64 {
        self.mtimecmp >> 32
    }
    pub fn mtimecmph_write(&mut self, data: u64) {
        self.mtimecmp = (self.mtimecmp & 0xffffffff) | (data << 32);
    }
}

pub struct Clint {
    harts: Vec<ClintHart>,
    mitme: CsrShare<u64>,
}

impl Clint {
    pub fn new() -> Self {
        Clint {
            harts: vec![],
            mitme: CsrShare::new(0.into()),
        }
    }
    // add a hart,and return the shared mitme
    pub fn add_hart(&mut self, xip_share: CsrShare<XipIn>) -> CsrShare<u64> {
        self.harts.push(ClintHart::new(xip_share));
        self.mitme.clone()
    }

    fn mtime_inc(&mut self, inc: usize) {
        let mut mitme = self.mitme.get();
        mitme += inc as u64;
        self.mitme.set(mitme);
    }

    pub fn tick(&mut self, inc: usize) {
        self.mtime_inc(inc);
        for hart in self.harts.iter_mut() {
            let level = self.mitme.get() >= hart.mtimecmp;
            let mut xip = hart.xip.get();
            xip.set_mtip(level);
            hart.xip.set(xip);
        }
    }
}

impl DeviceBase for Clint {
    fn do_read(&mut self, addr: u64, len: usize) -> u64 {
        match (addr, len) {
            (MSIP_BASE..=MSIP_END, 4) => {
                let hart_id = (addr - MSIP_BASE) / MSIP_PER_HART;
                let hart = &self.harts[hart_id as usize];
                hart.msip_read()
            }
            (MTIMECMP_BASE..=MTIMECMP_END, _) => {
                let hart_id = (addr - MTIMECMP_BASE) / MTIMECMP_PER_HART;
                let is_mtimecmph = (addr - MTIMECMP_BASE) % MTIMECMP_PER_HART == 4;
                let hart = &self.harts[hart_id as usize];

                match is_mtimecmph {
                    true => hart.mtimecmph_read(),
                    false => hart.mtimecmp_read(),
                }
            }
            (MTIME_BASE..=MTIME_BASE_END, _) => {
                let is_mtimeh = addr == MTIME_BASE + 4;
                let mitme: u64 = self.mitme.get();
                match is_mtimeh {
                    true => mitme >> 32,
                    false => mitme,
                }
            }
            _ => {
                panic!("clint read:{:x},{:x}", addr, len);
            }
        }
    }

    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        match (addr, len) {
            (MSIP_BASE..=MSIP_END, 4) => {
                let hart_id = (addr - MSIP_BASE) / MSIP_PER_HART;
                let hart = &mut self.harts[hart_id as usize];
                hart.msip_write(data);
            }
            (MTIMECMP_BASE..=MTIMECMP_END, _) => {
                let hart_id = (addr - MTIMECMP_BASE) / MTIMECMP_PER_HART;
                let is_mtimecmph = (addr - MTIMECMP_BASE) % MTIMECMP_PER_HART == 4;
                let hart = &mut self.harts[hart_id as usize];
                match is_mtimecmph {
                    true => hart.mtimecmph_write(data),
                    false => hart.mtimecmp_write(data),
                }
            }
            (MTIME_BASE, 8) => {
                self.mitme.set(data);
            }
            _ => {
                panic!("clint write:{:x},{:x},{:x}", addr, len, data);
            }
        };
        0
    }

    fn get_name(&self) -> &'static str {
        "Sifive CLINT"
    }
}

impl Default for Clint {
    fn default() -> Self {
        Self::new()
    }
}
