#![allow(dead_code)]
use enum_dispatch::enum_dispatch;
use super::{
    device_dram::DeviceDram, device_rtc::DeviceRTC, device_sifive_uart::DeviceSifiveUart,
    device_uart::DeviceUart,
};

#[cfg(feature = "device_sdl2")]
use super::{
    device_kb::DeviceKB, device_mouse::DeviceMouse, device_vga::DeviceVGA,
    device_vgactl::DeviceVGACTL,
};

pub const SIFIVE_UART_BASE: u64 = 0xc0000000;

pub const MEM_BASE: u64 = 0x80000000;
pub const DEVICE_BASE: u64 = 0xa0000000;
pub const SERIAL_PORT: u64 = DEVICE_BASE + 0x00003f8;
pub const RTC_ADDR: u64 = DEVICE_BASE + 0x0000048;
pub const KBD_ADDR: u64 = DEVICE_BASE + 0x0000060;
pub const MOUSE_ADDR: u64 = DEVICE_BASE + 0x0000070;
pub const FB_ADDR: u64 = DEVICE_BASE + 0x1000000;
pub const VGACTL_ADDR: u64 = DEVICE_BASE + 0x0000100;

#[enum_dispatch(DeviceEnume)]
pub trait DeviceBase {
    fn do_read(&mut self, addr: u64, len: usize) -> u64;
    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64;
    fn get_name(&self) -> &'static str;
    fn do_update(&mut self) {}
}

#[enum_dispatch]
pub enum DeviceEnume {
    DeviceDram,
    DeviceUart,
    DeviceRTC,
    DeviceSifiveUart,
    #[cfg(feature = "device_sdl2")]
    DeviceKB,
    #[cfg(feature = "device_sdl2")]
    DeviceMouse,
    #[cfg(feature = "device_sdl2")]
    DeviceVGACTL,
    #[cfg(feature = "device_sdl2")]
    DeviceVGA,
}
