mod bus;
mod cpu_core;
mod device_rtc;
mod device_trait;
mod device_uart;
mod dram;
mod gpr;
mod inst_base;
mod inst_decode;
mod inst_rv64i;
mod inst_rv64m;
mod inst_rv64z;
mod traptype;
use clap::Parser;

use crate::{
    bus::DeviceType,
    cpu_core::{CpuCore, CpuState},
    device_rtc::DeviceRTC,
    device_trait::{MEM_BASE, RTC_ADDR, SERIAL_PORT},
    device_uart::DeviceUart,
    dram::Dram,
};
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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, value_name = "FILE")]
    /// IMG bin ready to run
    img: String,
    #[arg(short, long, value_name = "FILE")]
    /// Write torture test signature to FILE
    signature: Option<String>,
}

fn main() {
    let args = Args::parse();

    let mut cpu = CpuCore::new();

    // device dram
    let mut mem = Box::new(Dram::new(128 * 1024 * 1024));
    mem.load_binary(&args.img);

    cpu.bus.add_device(DeviceType {
        start: MEM_BASE,
        len: mem.capacity as u64,
        instance: mem,
    });
    // device uart
    let uart = Box::new(DeviceUart::new());

    cpu.bus.add_device(DeviceType {
        start: SERIAL_PORT,
        len: 1,
        instance: uart,
    });
    // device rtc
    let rtc = Box::new(DeviceRTC::new());
    cpu.bus.add_device(DeviceType {
        start: RTC_ADDR,
        len: 8,
        instance: rtc,
    });


    // start sim
    cpu.cpu_state = CpuState::Running;
    let mut cycle = 0;
    loop {
        cpu.execute(1);
        cycle += 1;
        if cpu.cpu_state != CpuState::Running {
            break;
        }
    }

    println!("total:{cycle}");
    // dump signature
    args.signature
        .map(|x| cpu.dump_signature(&x))
        .unwrap_or_else(|| println!("no signature"));
}
