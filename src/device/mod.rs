pub mod device_dram;
pub mod device_rtc;
pub mod device_sifive_plic;
pub mod device_sifive_uart;
pub mod device_trait;
pub mod device_uart;
pub mod device_sifive_clint;
pub mod device_16550a;

#[cfg(feature = "device_sdl2")]
pub mod device_kb;
#[cfg(feature = "device_sdl2")]
pub mod device_mouse;
#[cfg(feature = "device_sdl2")]
pub mod device_vga;
#[cfg(feature = "device_sdl2")]
pub mod device_vgactl;


