pub mod device_16550a;
pub mod device_am_uart;
pub mod device_memory;
pub mod device_sifive_clint;
pub mod device_sifive_plic;
pub mod device_sifive_uart;
pub mod device_trait;

#[cfg(feature = "std")]
pub mod device_am_rtc;
cfg_if::cfg_if! {
    if #[cfg(all(feature = "device_sdl2", feature = "std"))] {
        pub mod device_am_kb;
        pub mod device_am_mouse;
        pub mod device_am_vga;
        pub mod device_am_vgactl;
    }
}
