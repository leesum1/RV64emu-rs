const MTIMECMP_OFFSET: u64 = 0x4000;
const MTIME_OFFSET: u64 = 0xBFF8;
const MSIP_OFFSET: u64 = 0x0;

pub struct DeviceClint {
    pub start: u64,
    pub len: u64,
    pub instance: Clint,
    pub name: &'static str,
}

pub struct Clint {
    mtime: u64,
    mtimecmp: u64,
}

impl Clint {
    pub fn new() -> Self {
        Clint {
            mtime: 0,
            mtimecmp: 0,
        }
    }
    pub fn do_read(&self, addr: u64, len: usize) -> u64 {
        match (addr, len) {
            (MSIP_OFFSET, 4) => 0,
            (MTIME_OFFSET, 8) => self.mtime,
            (MTIMECMP_OFFSET, 8) => self.mtimecmp,
            (offset, _len) => panic!("Read Clint offset:{offset} err!"),
        }
    }
    pub fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        match (addr, len) {
            (MTIME_OFFSET, 8) => self.mtime = data,
            (MTIMECMP_OFFSET, 8) => self.mtimecmp = data,
            (offset, _len) => panic!("Write Clint offset:{offset} err!"),
        };
        data
    }

    pub fn do_update(&mut self) {
        self.mtime += 2;
    }

    pub fn is_interrupt(&self) -> bool {
        self.mtime >= self.mtimecmp
    }
}
