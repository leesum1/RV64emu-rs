pub mod tools;

extern crate riscv64_emu;

use clap::Parser;
use riscv64_emu::tools::FifoUnbounded;
#[allow(unused_imports)]
use riscv64_emu::tools::Fifobounded;

use std::io::{stdin, Write};
#[allow(unused_imports)]
use std::{
    cell::Cell,
    io::{self, Read},
    num::NonZeroUsize,
    process,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use log::{debug, info, LevelFilter};
use riscv64_emu::{device::device_16550a::Device16550aUART, rvsim::RVsim};

cfg_if::cfg_if! {
    if #[cfg(feature = "device_sdl2")]{
        use riscv64_emu::device::{
            device_am_kb::{DeviceKB, DeviceKbItem},
            device_am_mouse::{DeviceMouse, DeviceMouseItem},
            device_trait::{FB_ADDR, KBD_ADDR, MOUSE_ADDR, VGACTL_ADDR},
            device_am_vga::DeviceVGA,
            device_am_vgactl::DeviceVGACTL,
        };
        use sdl2::{
            event::Event,
            keyboard::{Keycode, Scancode},
        };
    }
}

#[cfg(feature = "rv_debug_trace")]
use crate::riscv64_emu::trace::traces::Traces;
use crate::tools::RVmutex;
use crate::{
    riscv64_emu::device::{
        device_am_rtc::DeviceRTC,
        device_am_uart::DeviceUart,
        device_memory::DeviceMemory,
        device_sifive_plic::SIFIVE_UART_IRQ,
        device_sifive_uart::DeviceSifiveUart,
        device_trait::DeviceBase,
        device_trait::{MEM_BASE, RTC_ADDR, SERIAL_PORT, SIFIVE_UART_BASE},
    },
    riscv64_emu::rv64core::bus::{Bus, DeviceType},
    riscv64_emu::rv64core::cpu_core::CpuCoreBuild,
};

// /* 各个设备地址 */
// -------------Device Tree MAP-------------
// name:CLINT           Area:0X02000000-->0X02010000,len:0X00010000
// name:PLIC            Area:0X0C000000-->0X10000000,len:0X04000000
// name:DRAM            Area:0X80000000-->0X88000000,len:0X08000000
// name:UART            Area:0XA00003F8-->0XA00003F9,len:0X00000001
// name:RTC             Area:0XA0000048-->0XA0000050,len:0X00000008
// name:VGA_CTL         Area:0XA0000100-->0XA0000108,len:0X00000008
// name:VGA_FB          Area:0XA1000000-->0XA1075300,len:0X00075300
// name:KeyBorad_AM     Area:0XA0000060-->0XA0000068,len:0X00000008
// name:Mouse           Area:0XA0000070-->0XA0000080,len:0X00000010
// name:Sifive_Uart     Area:0XC0000000-->0XC0001000,len:0X00001000

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, value_name = "FILE")]
    /// IMG bin copy to ram
    img: Option<String>,
    #[arg(long, value_name = "FILE")]
    /// IMG bin copy to xipflash
    xipflash: Option<String>,
    #[arg(long, value_name = "HEX")]
    /// the first instruction address,default:0x80000000
    boot_pc: Option<String>,
    #[arg(short, long, value_name = "USIZE")]
    /// Number of harts,default:1
    num_harts: Option<usize>,
    #[arg(short, long, value_name = "FILE")]
    /// optional:Write torture test signature to FILE
    signature: Option<String>,
}

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .init()
        .unwrap();
    let args = Args::parse();

    let signal_term = Arc::new(AtomicBool::new(false));
    let signal_term_cpucore = signal_term.clone();

    #[allow(unused_variables)]
    #[allow(clippy::redundant_clone)]
    let signal_term_sdl_event = signal_term.clone();
    #[allow(unused_variables)]
    #[allow(clippy::redundant_clone)]
    let signal_term_uart = signal_term.clone();

    let bus_u: RVmutex<Bus> = RVmutex::new(Bus::new().into());

    // device dram len:0X08000000
    let mem = DeviceMemory::new(0x0800000);

    bus_u.borrow_mut().add_device(DeviceType {
        start: MEM_BASE,
        len: mem.size() as u64,
        instance: Box::new(mem),
        name: "RAM",
    });

    // device dtcm
    let dtcm = DeviceMemory::new(128 * 1024 * 1024);
    bus_u.borrow_mut().add_device(DeviceType {
        start: 0x9000_0000,
        len: dtcm.size() as u64,
        instance: Box::new(dtcm),
        name: "DTCM",
    });

    // device flash len:0X01000000
    let falsh = DeviceMemory::new(128 * 1024 * 1024);

    bus_u.borrow_mut().add_device(DeviceType {
        start: 0x3000_0000,
        len: falsh.size() as u64,
        instance: Box::new(falsh),
        name: "XIPFLASH",
    });

    // let (uart_getc_tx, uart_getc_rx) = crossbeam_channel::bounded::<u8>(64);
    // let (uart_putc_tx, uart_putc_rx) = crossbeam_channel::bounded::<u8>(4096 * 2);

    let uart_tx_fifo = FifoUnbounded::new(crossbeam_queue::SegQueue::<u8>::new());
    let uart_rx_fifo = FifoUnbounded::new(crossbeam_queue::SegQueue::<u8>::new());

    let rx_fifo = uart_rx_fifo.clone();

    thread::spawn(move || loop {
        let mut buf = [0; 1];
        if let Ok(n) = stdin().read(&mut buf) {
            if n > 1 {
                panic!("Read {} characters into a 1 byte buffer", n);
            }
            if n == 1 {
                // uart_getc_tx.send(buf[0]).unwrap();
                rx_fifo.push(buf[0]);
            }
            // Nothing needs to be sent for n == 0
        }
        // std::thread::sleep(Duration::from_millis(100));
    });

    let tx_fifo = uart_tx_fifo.clone();
    thread::spawn(move || loop {
        while !tx_fifo.is_empty() {
            if let Some(c) = tx_fifo.pop() {
                print!("{}", c as char)
            }
        }

        io::stdout().flush().unwrap();
        std::thread::sleep(Duration::from_millis(50));
    });

    // device uart
    let uart = DeviceUart::new(uart_tx_fifo.clone());

    bus_u.borrow_mut().add_device(DeviceType {
        start: SERIAL_PORT,
        len: 1,
        instance: Box::new(uart),
        name: "AM_UART",
    });
    // device 16650_uart
    let device_16650_uart = Device16550aUART::new(uart_tx_fifo.clone(), uart_rx_fifo.clone());

    bus_u.borrow_mut().add_device(DeviceType {
        start: 0x1000_0000,
        len: 0x1000,
        instance: Box::new(device_16650_uart),
        name: "16550a_uart",
    });

    // device sifive_uart
    let device_sifive_uart = DeviceSifiveUart::new(uart_tx_fifo, uart_rx_fifo);

    bus_u
        .borrow_mut()
        .plic
        .instance
        .register_irq_source(SIFIVE_UART_IRQ, Rc::clone(&device_sifive_uart.irq_pending));

    bus_u.borrow_mut().add_device(DeviceType {
        start: SIFIVE_UART_BASE,
        len: 0x1000,
        instance: Box::new(device_sifive_uart),
        name: "Sifive_Uart",
    });

    // device rtc
    let rtc = DeviceRTC::new();
    let device_name = rtc.get_name();

    bus_u.borrow_mut().add_device(DeviceType {
        start: RTC_ADDR,
        len: 8,
        instance: Box::new(rtc),
        name: device_name,
    });

    #[cfg(feature = "device_sdl2")]
    create_sdl2_devices(bus_u.clone(), signal_term_sdl_event);

    // show device address map
    debug!("{0}", bus_u.borrow_mut());

    let hart_num: usize = args.num_harts.unwrap_or(1);

    let boot_pc = if let Some(x) = args.boot_pc.as_ref() {
        u64::from_str_radix(
            x.trim_start_matches(|c| c == '0' || c == 'x' || c == 'X'),
            16,
        )
        .unwrap()
    } else {
        0x8000_0000
    };

    info!("boot_pc:0x{:x}", boot_pc);
    let mut hart_vec = Vec::new();
    #[allow(unused_mut)]
    let mut trace_handle: Vec<thread::JoinHandle<()>> = Vec::new();
    // create hart according to the number of harts
    for hart_id in 0..hart_num {
        cfg_if::cfg_if! {
                if #[cfg(feature = "rv_debug_trace")] {
                let (trace_tx, trace_rx) = crossbeam_channel::bounded(8096);
                let cpu: riscv64_emu::rv64core::cpu_core::CpuCore = CpuCoreBuild::new(bus_u.clone())
                    .with_boot_pc(boot_pc)
                    .with_hart_id(hart_id)
                    .with_trace(trace_tx)
                    .with_smode(true)
                    .build();
                hart_vec.push(cpu);
                let mut trace_log = Traces::new(0, trace_rx);
                let term_sig_trace = signal_term.clone();

                // create another thread to handle trace log
                let trace_thread = thread::spawn(move || {
                    while !term_sig_trace.load(Ordering::Relaxed) {
                        trace_log.run();
                    }
                });

                trace_handle.push(trace_thread);
            } else {
                let hart: riscv64_emu::rv64core::cpu_core::CpuCore = CpuCoreBuild::new(bus_u.clone())
                    .with_boot_pc(boot_pc)
                    .with_hart_id(hart_id)
                    .with_smode(true)
                    .build();
                hart_vec.push(hart);
            }
        };
    }

    // create another thread to simmulate the harts
    // while the main thread is used to handle sdl events
    // which will be send to the coresponding devices through ring_channel
    let cpu_main = thread::spawn(move || {
        let mut sim = RVsim::new(hart_vec);
        if let Some(ram_img) = args.img {
            sim.load_image(&ram_img);
        }
        if let Some(signature_file) = args.signature {
            sim.set_signature_file(signature_file);
        }

        sim.run();

        signal_term_cpucore.store(true, Ordering::Release);
    });

    // wait for all trace threads to finish
    for handle in trace_handle {
        handle.join().unwrap();
    }
    cpu_main.join().unwrap();
}

#[cfg(feature = "device_sdl2")]
fn send_key_event(tx: &Fifobounded<DeviceKbItem>, val: Scancode, keydown: bool) {
    tx.force_push(DeviceKbItem {
        scancode: val,
        is_keydown: keydown,
    });
}
#[cfg(feature = "device_sdl2")]
fn create_sdl2_devices(bus_u: RVmutex<Bus>, signal_term_sdl_event: Arc<AtomicBool>) {
    /*--------init sdl --------*/
    // subsequnt devices are base on sdl2 api
    // 1. device vga
    // 2. device kb
    // 3. device mouse
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let event_system = sdl_context.event().expect("fail");
    let event_system: &'static sdl2::EventSubsystem = Box::leak(Box::new(event_system));

    let window = video_subsystem
        .window("rust-sdl2 demo: Video", 800, 600)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())
        .unwrap();

    let mut canvas = window.into_canvas().build().expect("canvas err");
    canvas.set_scale(2.0, 2.0).unwrap();

    // device vgactl
    let vgactl_msg = Rc::new(Cell::new(false));

    let vgactl = DeviceVGACTL::new(vgactl_msg.clone());

    let device_name = vgactl.get_name();
    bus_u.borrow_mut().add_device(DeviceType {
        start: VGACTL_ADDR,
        len: 8,
        instance: Box::new(vgactl),
        name: device_name,
    });

    // device vga
    let vga = DeviceVGA::new(canvas, vgactl_msg);
    let device_name = vga.get_name();
    bus_u.borrow_mut().add_device(DeviceType {
        start: FB_ADDR,
        len: DeviceVGA::get_size() as u64,
        instance: Box::new(vga),
        name: device_name,
    });

    let kb_am_fifo = Fifobounded::<DeviceKbItem>::new(crossbeam_queue::ArrayQueue::new(16));

    let kb_sdl_fifo =
        Fifobounded::<sdl2::keyboard::Keycode>::new(crossbeam_queue::ArrayQueue::new(16));

    let device_kb = DeviceKB::new(kb_am_fifo.clone(), kb_sdl_fifo.clone());

    let device_name = device_kb.get_name();
    bus_u.borrow_mut().add_device(DeviceType {
        start: KBD_ADDR,
        len: 8,
        instance: Box::new(device_kb),
        name: device_name,
    });
    // device mouse
    // let (mouse_sdl_tx, mouse_sdl_rx): (Fifobounded<DeviceMouseItem>, Fifobounded<DeviceMouseItem>) =
    //     ring_channel(NonZeroUsize::new(1).unwrap());

    let mouse_fifo = Fifobounded::<DeviceMouseItem>::new(crossbeam_queue::ArrayQueue::new(16));

    let device_mouse = DeviceMouse::new(mouse_fifo.clone());

    bus_u.borrow_mut().add_device(DeviceType {
        start: MOUSE_ADDR,
        len: 16,
        instance: Box::new(device_mouse),
        name: "Mouse",
    });

    handle_sdl_event(
        signal_term_sdl_event,
        event_system,
        kb_am_fifo,
        kb_sdl_fifo,
        mouse_fifo,
    );
}
#[cfg(feature = "device_sdl2")]
fn handle_sdl_event(
    signal_term: Arc<AtomicBool>,
    static_event: &'static sdl2::EventSubsystem,
    kb_am_tx: Fifobounded<DeviceKbItem>,
    kb_sdl_tx: Fifobounded<Keycode>,
    mouse_sdl_tx: Fifobounded<DeviceMouseItem>,
) {
    thread::spawn(move || {
        let _event_system = static_event;
        let _sdl_context = _event_system.sdl();
        let mut event_pump = _sdl_context.event_pump().expect("fail to get event_pump");
        while !signal_term.load(Ordering::Relaxed) {
            let mouse_state = event_pump.mouse_state();

            mouse_sdl_tx.force_push(DeviceMouseItem {
                x: (mouse_state.x() / 2) as u32,
                y: (mouse_state.y() / 2) as u32,
                mouse_btn_state: mouse_state.to_sdl_state(),
            });

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => process::exit(0),
                    Event::KeyUp {
                        scancode: Some(val),
                        ..
                    } => send_key_event(&kb_am_tx, val, false),

                    Event::KeyDown {
                        scancode: Some(val),
                        keycode: Some(sdl_key_code),
                        ..
                    } => {
                        send_key_event(&kb_am_tx, val, true);
                        kb_sdl_tx.force_push(sdl_key_code);
                    }

                    _ => (),
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
}
