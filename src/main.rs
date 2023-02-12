mod bus;
mod cpu_core;
mod device_dram;
mod device_kb;
mod device_rtc;
mod device_trait;
mod device_uart;
mod device_vga;
mod device_vgactl;
mod gpr;
mod inst_base;
mod inst_decode;
mod inst_rv64i;
mod inst_rv64m;
mod inst_rv64z;
mod traptype;

use std::{num::NonZeroUsize, thread, time::Duration};

use clap::Parser;
use ring_channel::*;
use sdl2::event::Event;

use crate::{
    bus::DeviceType,
    cpu_core::{CpuCore, CpuState},
    device_dram::DeviceDram,
    device_kb::{DeviceKB, DeviceKbItem},
    device_rtc::DeviceRTC,
    device_trait::{DeviceBase, FB_ADDR, KBD_ADDR, MEM_BASE, RTC_ADDR, SERIAL_PORT, VGACTL_ADDR},
    device_uart::DeviceUart,
    device_vga::DeviceVGA,
    device_vgactl::DeviceVGACTL,
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
    let mut mem = Box::new(DeviceDram::new(128 * 1024 * 1024));
    mem.load_binary(&args.img);
    let device_name = mem.get_name();

    cpu.bus.add_device(DeviceType {
        start: MEM_BASE,
        len: mem.capacity as u64,
        instance: mem,
        name: device_name,
    });

    // device uart
    let uart = Box::new(DeviceUart::new());
    let device_name = uart.get_name();

    cpu.bus.add_device(DeviceType {
        start: SERIAL_PORT,
        len: 1,
        instance: uart,
        name: device_name,
    });

    // device rtc
    let rtc = Box::new(DeviceRTC::new());
    let device_name = rtc.get_name();

    cpu.bus.add_device(DeviceType {
        start: RTC_ADDR,
        len: 8,
        instance: rtc,
        name: device_name,
    });

    // device vga
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("rust-sdl2 demo: Video", 800, 600)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())
        .unwrap();

    let canvas = window.into_canvas().build().expect("canvas err");

    let vga = Box::new(DeviceVGA::new(canvas));
    let device_name = vga.get_name();

    cpu.bus.add_device(DeviceType {
        start: FB_ADDR,
        len: DeviceVGA::get_size() as u64,
        instance: vga,
        name: device_name,
    });

    // device vgactl
    let vgactl = Box::new(DeviceVGACTL::new());
    let device_name = vgactl.get_name();

    cpu.bus.add_device(DeviceType {
        start: VGACTL_ADDR,
        len: 8,
        instance: vgactl,
        name: device_name,
    });

    // device kb
    let event_system = sdl_context.event().expect("fail");
    // Contrived, but definitely safe.
    // Another way to get a 'static subsystem reference is through the use of a global,
    // i.e. via `lazy_static`. This is also the *only* way to actually use a subsystem on another thread.
    let static_event: &'static _ = Box::leak(Box::new(event_system));

    // 创建消息通道，tx是生产者，rx是消费者
    let (mut tx, rx) = ring_channel(NonZeroUsize::new(16).unwrap());

    let device_kb = Box::new(DeviceKB::new(rx));
    let device_name = device_kb.get_name();

    cpu.bus.add_device(DeviceType {
        start: KBD_ADDR,
        len: 4,
        instance: device_kb,
        name: device_name,
    });

    println!("{0}", cpu.bus);

    thread::spawn(move || -> ! {
        let _event_system = static_event;
        let _sdl_context = _event_system.sdl();
        let mut event_pump = _sdl_context.event_pump().expect("fail to get event_pump");
        loop {
            for event in event_pump.poll_iter() {
                match event {
                    // skip mouse motion intentionally because of the verbose it might cause.
                    Event::MouseMotion { .. } => {}
                    Event::KeyUp {
                        timestamp: _,
                        window_id: _,
                        keycode: _,
                        scancode: Some(val),
                        keymod: _,
                        repeat: _,
                    } => {
                        tx.send(DeviceKbItem {
                            scancode: val,
                            is_keydown: false,
                        })
                        .expect("KeyDown send err");
                    }
                    Event::KeyDown {
                        timestamp: _,
                        window_id: _,
                        keycode: _,
                        scancode: Some(val),
                        keymod: _,
                        repeat: _,
                    } => {
                        tx.send(DeviceKbItem {
                            scancode: val,
                            is_keydown: true,
                        })
                        .expect("KeyDown send err");
                    }

                    _ => (),
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
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
