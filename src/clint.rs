const MTIMECMP_OFFSET: u64 = 0x4000;
const MTIME_OFFSET: u64 = 0xBFF8;

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
        assert_eq!(len, 8);

        match addr {
            MTIME_OFFSET => self.mtime,
            MTIMECMP_OFFSET => self.mtimecmp,
            offset => panic!("Read Clint offset:{offset} err!"),
        }
    }
    pub fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        assert_eq!(len, 8);
        match addr {
            MTIME_OFFSET => self.mtime = data,
            MTIMECMP_OFFSET => self.mtimecmp = data,
            offset => panic!("Write Clint offset:{offset} err!"),
        };
        data
    }

    pub fn do_update(&mut self) {
        self.mtime = self.mtime.wrapping_add(1);
    }

    pub fn is_interrupt(&self) -> bool {
        self.mtime >= self.mtimecmp
    }
}
