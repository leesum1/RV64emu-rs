pub const MEM_BASE: u64 = 0x80000000;
pub const DEVICE_BASE: u64 = 0xa0000000;
pub const SERIAL_PORT: u64 = DEVICE_BASE + 0x00003f8;
pub const RTC_ADDR: u64 = DEVICE_BASE + 0x0000048;

pub trait DeviceBase {
    fn do_read(&mut self, addr: u64, len: usize) -> u64;
    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64;
    fn do_update(&mut self) {
        println!("do_update");
    }
}
