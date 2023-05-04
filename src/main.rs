extern crate riscv64_emu;

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
use std::{
    io::{stdin, Write},
    sync::Mutex,
};

use clap::Parser;
use log::{debug, info, LevelFilter};
use riscv64_emu::device::device_16550a::Device16550aUART;

cfg_if::cfg_if! {
    if #[cfg(feature = "device_sdl2")]{
        use riscv64_emu::device::{
            device_kb::{DeviceKB, DeviceKbItem},
            device_mouse::{DeviceMouse, DeviceMouseItem},
            device_trait::{FB_ADDR, KBD_ADDR, MOUSE_ADDR, VGACTL_ADDR},
            device_vga::DeviceVGA,
            device_vgactl::DeviceVGACTL,
        };
        use sdl2::{
            event::Event,
            keyboard::{Keycode, Scancode},
        };
        use ring_channel::*;
        use riscv64_emu::rv64core::cpu_core::CpuCore;
    }
}

#[cfg(feature = "rv_debug_trace")]
use crate::riscv64_emu::trace::traces::Traces;
use crate::{
    riscv64_emu::device::{
        device_dram::DeviceDram,
        device_rtc::DeviceRTC,
        device_sifive_plic::SIFIVE_UART_IRQ,
        device_sifive_uart::DeviceSifiveUart,
        device_trait::DeviceBase,
        device_trait::{MEM_BASE, RTC_ADDR, SERIAL_PORT, SIFIVE_UART_BASE},
        device_uart::DeviceUart,
    },
    riscv64_emu::rv64core::bus::{Bus, DeviceType},
    riscv64_emu::rv64core::cpu_core::{CpuCoreBuild, CpuState},
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
    let signal_term_sdl_event = signal_term.clone();
    #[allow(unused_variables)]
    let signal_term_uart = signal_term.clone();

    let bus_u: Arc<Mutex<Bus>> = Arc::new(Mutex::new(Bus::new()));

    // device dram len:0X08000000
    let mut mem = DeviceDram::new(128 * 1024 * 1024);

    if let Some(ram_img) = args.img {
        mem.load_binary(&ram_img);
    }
    bus_u.lock().unwrap().add_device(DeviceType {
        start: MEM_BASE,
        len: mem.capacity as u64,
        instance: mem.into(),
        name: "RAM",
    });

    // device dtcm
    let dtcm = DeviceDram::new(128 * 1024 * 1024);
    bus_u.lock().unwrap().add_device(DeviceType {
        start: 0x9000_0000,
        len: dtcm.capacity as u64,
        instance: dtcm.into(),
        name: "DTCM",
    });

    // device flash len:0X01000000
    let mut falsh = DeviceDram::new(128 * 1024 * 1024);
    if let Some(xipflash) = &args.xipflash {
        falsh.load_binary(xipflash);
    }
    bus_u.lock().unwrap().add_device(DeviceType {
        start: 0x3000_0000,
        len: falsh.capacity as u64,
        instance: falsh.into(),
        name: "XIPFLASH",
    });

    let (uart_getc_tx, uart_getc_rx) = crossbeam_channel::bounded::<u8>(64);
    let (uart_putc_tx, uart_putc_rx) = crossbeam_channel::bounded::<u8>(2048);
    thread::spawn(move || loop {
        let mut buf = [0; 1];
        if let Ok(n) = stdin().read(&mut buf) {
            if n > 1 {
                panic!("Read {} characters into a 1 byte buffer", n);
            }
            if n == 1 {
                uart_getc_tx.send(buf[0]).unwrap();
            }
            // Nothing needs to be sent for n == 0
        }
        std::thread::sleep(Duration::from_millis(100));
    });

    thread::spawn(move || loop {
        uart_putc_rx.try_iter().for_each(|c| {
            print!("{}", c as char);
        });
        io::stdout().flush().unwrap();
        std::thread::sleep(Duration::from_millis(50));
    });

    // device uart
    let uart = DeviceUart::new();

    bus_u.lock().unwrap().add_device(DeviceType {
        start: SERIAL_PORT,
        len: 1,
        instance: uart.into(),
        name: "AM_UART",
    });
    // device 16650_uart
    let device_16650_uart = Device16550aUART::new(uart_putc_tx.clone(), uart_getc_rx.clone());

    bus_u.lock().unwrap().add_device(DeviceType {
        start: 0x1000_0000,
        len: 0x1000,
        instance: device_16650_uart.into(),
        name: "16550a_uart",
    });

    // device sifive_uart
    let device_sifive_uart = DeviceSifiveUart::new(uart_putc_tx.clone(), uart_getc_rx.clone());

    bus_u
        .lock()
        .unwrap()
        .plic
        .instance
        .register_irq_source(SIFIVE_UART_IRQ, Rc::clone(&device_sifive_uart.irq_pending));

    bus_u.lock().unwrap().add_device(DeviceType {
        start: SIFIVE_UART_BASE,
        len: 0x1000,
        instance: device_sifive_uart.into(),
        name: "Sifive_Uart",
    });

    // device rtc
    let rtc = DeviceRTC::new();
    let device_name = rtc.get_name();

    bus_u.lock().unwrap().add_device(DeviceType {
        start: RTC_ADDR,
        len: 8,
        instance: rtc.into(),
        name: device_name,
    });

    #[cfg(feature = "device_sdl2")]
    create_sdl2_devices(bus_u.clone(), signal_term_sdl_event);

    // show device address map
    debug!("{0}", bus_u.lock().unwrap());

    let hart_num = match args.num_harts {
        Some(num) => num,
        None => 1,
    };

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

    #[cfg(feature = "expr_mutithread")]
    for mut hart in hart_vec {
        let cpu_siganl = signal_term_cpucore.clone();
        thread::spawn(move || {
            hart.cpu_state = CpuState::Running;

            while hart.cpu_state == CpuState::Running {
                hart.execute(1000);
            }
            cpu_siganl.store(true, Ordering::Relaxed);
        });
    }

    #[cfg(feature = "expr_mutithread")]
    let bus_thread = thread::spawn(move || {
        while !signal_term.load(Ordering::Relaxed) {
            {
                bus_u.lock().unwrap().update();
                bus_u.lock().unwrap().clint.instance.tick(5000 / 100);
                bus_u.lock().unwrap().plic.instance.tick();
            }
            thread::sleep(Duration::from_millis(50));
        }
    });

    // create another thread to simmulate the harts
    // while the main thread is used to handle sdl events
    // which will be send to the coresponding devices through ring_channel
    #[cfg(not(feature = "expr_mutithread"))]
    let cpu_main = thread::spawn(move || {
        // start sim
        hart_vec.iter_mut().for_each(|x| {
            x.cpu_state = CpuState::Running;
        });

        let mut cycle: u64 = 0;

        // if all harts are in running state, then continue to execute
        while hart_vec
            .iter()
            .all(|hart| hart.cpu_state == CpuState::Running)
        {
            hart_vec.iter_mut().for_each(|hart| {
                hart.execute(5000);
                bus_u.lock().unwrap().update();
                bus_u.lock().unwrap().clint.instance.tick(5000 / 100);
                bus_u.lock().unwrap().plic.instance.tick();
            });
            // not accurate, but enough for now
            cycle += 5000;
        }
        println!("total:{cycle}");
        // send signal to stop the trace thread
        signal_term_cpucore.store(true, Ordering::Relaxed);
    });

    for handle in trace_handle {
        handle.join().unwrap();
    }

    #[cfg(feature = "expr_mutithread")]
    bus_thread.join().unwrap();
    #[cfg(not(feature = "expr_mutithread"))]
    cpu_main.join().unwrap();
}

#[cfg(feature = "device_sdl2")]
fn send_key_event(tx: &mut RingSender<DeviceKbItem>, val: Scancode, keydown: bool) {
    tx.send(DeviceKbItem {
        scancode: val,
        is_keydown: keydown,
    })
    .expect("Key event send error");
}
#[cfg(feature = "device_sdl2")]
fn create_sdl2_devices(bus_u: Arc<Mutex<Bus>>, signal_term_sdl_event: Arc<AtomicBool>) {
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
    bus_u.lock().unwrap().add_device(DeviceType {
        start: VGACTL_ADDR,
        len: 8,
        instance: vgactl.into(),
        name: device_name,
    });

    // device vga
    let vga = DeviceVGA::new(canvas, vgactl_msg);
    let device_name = vga.get_name();
    bus_u.lock().unwrap().add_device(DeviceType {
        start: FB_ADDR,
        len: DeviceVGA::get_size() as u64,
        instance: vga.into(),
        name: device_name,
    });

    // device kb

    let (kb_am_tx, kb_am_rx): (RingSender<DeviceKbItem>, RingReceiver<DeviceKbItem>) =
        ring_channel(NonZeroUsize::new(16).unwrap());

    let (kb_sdl_tx, kb_sdl_rx): (
        RingSender<sdl2::keyboard::Keycode>,
        RingReceiver<sdl2::keyboard::Keycode>,
    ) = ring_channel(NonZeroUsize::new(16).unwrap());

    let device_kb = DeviceKB::new(kb_am_rx, kb_sdl_rx);
    let device_name = device_kb.get_name();

    bus_u.lock().unwrap().add_device(DeviceType {
        start: KBD_ADDR,
        len: 8,
        instance: device_kb.into(),
        name: device_name,
    });
    // device mouse
    let (mouse_sdl_tx, mouse_sdl_rx): (RingSender<DeviceMouseItem>, RingReceiver<DeviceMouseItem>) =
        ring_channel(NonZeroUsize::new(1).unwrap());
    let device_mouse = DeviceMouse::new(mouse_sdl_rx);

    bus_u.lock().unwrap().add_device(DeviceType {
        start: MOUSE_ADDR,
        len: 16,
        instance: device_mouse.into(),
        name: "Mouse",
    });

    handle_sdl_event(
        signal_term_sdl_event,
        event_system,
        kb_am_tx,
        kb_sdl_tx,
        mouse_sdl_tx,
    );
}
#[cfg(feature = "device_sdl2")]
fn handle_sdl_event(
    signal_term: Arc<AtomicBool>,
    static_event: &'static sdl2::EventSubsystem,
    mut kb_am_tx: RingSender<DeviceKbItem>,
    mut kb_sdl_tx: RingSender<Keycode>,
    mut mouse_sdl_tx: RingSender<DeviceMouseItem>,
) {
    thread::spawn(move || {
        let _event_system = static_event;
        let _sdl_context = _event_system.sdl();
        let mut event_pump = _sdl_context.event_pump().expect("fail to get event_pump");
        while !signal_term.load(Ordering::Relaxed) {
            let mouse_state = event_pump.mouse_state();

            mouse_sdl_tx
                .send(DeviceMouseItem {
                    x: (mouse_state.x() / 2) as u32,
                    y: (mouse_state.y() / 2) as u32,
                    mouse_btn_state: mouse_state.to_sdl_state(),
                })
                .expect("Mouse event send error");

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => process::exit(0),
                    Event::KeyUp {
                        scancode: Some(val),
                        ..
                    } => send_key_event(&mut kb_am_tx, val, false),

                    Event::KeyDown {
                        scancode: Some(val),
                        keycode: Some(sdl_key_code),
                        ..
                    } => {
                        send_key_event(&mut kb_am_tx, val, true);
                        kb_sdl_tx.send(sdl_key_code).unwrap();
                    }

                    _ => (),
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
}
