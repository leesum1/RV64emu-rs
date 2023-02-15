// /* 各个设备地址 */
// #define MEM_BASE 0x80000000
// #define DEVICE_BASE 0xa0000000
// #define MMIO_BASE 0xa0000000
// #define SERIAL_PORT     (DEVICE_BASE + 0x00003f8)
// #define KBD_ADDR        (DEVICE_BASE + 0x0000060)
// #define RTC_ADDR        (DEVICE_BASE + 0x0000048)
// #define VGACTL_ADDR     (DEVICE_BASE + 0x0000100)
// #define AUDIO_ADDR      (DEVICE_BASE + 0x0000200)
// #define DISK_ADDR       (DEVICE_BASE + 0x0000300)
// #define FB_ADDR         (MMIO_BASE   + 0x1000000)
// #define AUDIO_SBUF_ADDR (MMIO_BASE   + 0x1200000)

pub const MEM_BASE: u64 = 0x80000000;
pub const DEVICE_BASE: u64 = 0xa0000000;
pub const SERIAL_PORT: u64 = DEVICE_BASE + 0x00003f8;
pub const KBD_ADDR: u64 = DEVICE_BASE + 0x0000060;
pub const RTC_ADDR: u64 = DEVICE_BASE + 0x0000048;
pub const FB_ADDR: u64 = DEVICE_BASE + 0x1000000;
pub const VGACTL_ADDR: u64 = DEVICE_BASE + 0x0000100;

pub trait DeviceBase {
    fn do_read(&mut self, addr: u64, len: usize) -> u64;
    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64;
    fn get_name(&self) -> &'static str;
    fn do_update(&mut self) {}
}
