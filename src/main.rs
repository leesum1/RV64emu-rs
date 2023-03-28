mod bus;
mod cpu_core;
mod cpu_icache;
mod csr_regs;
mod csr_regs_define;
mod device;
mod gpr;
mod inst;
mod inst_decode;
mod mmu;
mod sifive_clint;
mod sv39;
mod trace;
mod traptype;

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
    thread::{self, JoinHandle},
    time::Duration,
};

use clap::Parser;

use ring_channel::*;

use sdl2::{
    event::Event,
    keyboard::{Keycode, Scancode},
};

use crate::{
    bus::DeviceType,
    cpu_core::{CpuCore, CpuState},
    device::{
        device_dram::DeviceDram,
        device_kb::{DeviceKB, DeviceKbItem},
        device_mouse::{DeviceMouse, DeviceMouseItem},
        device_rtc::DeviceRTC,
        device_sifive_uart::DeviceSifiveUart,
        device_trait::DeviceBase,
        device_trait::{
            FB_ADDR, KBD_ADDR, MEM_BASE, MOUSE_ADDR, RTC_ADDR, SERIAL_PORT, SIFIVE_UART_BASE,
            VGACTL_ADDR,
        },
        device_uart::DeviceUart,
        device_vga::DeviceVGA,
        device_vgactl::DeviceVGACTL,
    },
    trace::traces::Traces,
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
    let signal_term = Arc::new(AtomicBool::new(false));
    let signal_term_cpucore = signal_term.clone();
    let signal_term_trace = signal_term.clone();
    let signal_term_uart = signal_term;

    let (trace_tx, trace_rx) = crossbeam_channel::unbounded();
    let mut trace_log = Traces::new(trace_rx);

    let mut cpu = if cfg!(feature = "rv_debug_trace") {
        println!("Enabling debug tracing");
        CpuCore::new(Some(trace_tx))
    } else {
        println!("Disabling debug tracing");
        CpuCore::new(None)
    };

    // device dram
    let mut mem = DeviceDram::new(128 * 1024 * 1024);
    mem.load_binary(&args.img);
    let device_name = mem.get_name();

    cpu.mmu.bus.add_device(DeviceType {
        start: MEM_BASE,
        len: mem.capacity as u64,
        instance: mem.into(),
        name: device_name,
    });

    // device uart
    let uart = DeviceUart::new();
    let device_name = uart.get_name();

    cpu.mmu.bus.add_device(DeviceType {
        start: SERIAL_PORT,
        len: 1,
        instance: uart.into(),
        name: device_name,
    });

    // device rtc
    let rtc = DeviceRTC::new();
    let device_name = rtc.get_name();

    cpu.mmu.bus.add_device(DeviceType {
        start: RTC_ADDR,
        len: 8,
        instance: rtc.into(),
        name: device_name,
    });

    /*--------init sdl --------*/
    // subsequnt devices are base on sdl2 api
    // 1. device vga
    // 2. device kb
    // 3. device mouse
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let event_system = sdl_context.event().expect("fail");

    let window = video_subsystem
        .window("rust-sdl2 demo: Video", 800, 600)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())
        .unwrap();

    let mut canvas = window.into_canvas().build().expect("canvas err");
    canvas.set_scale(2.0, 2.0).unwrap();
    /*--------init sdl --------*/

    // device vgactl

    let vgactl_msg = Rc::new(Cell::new(false));

    let vgactl = DeviceVGACTL::new(vgactl_msg.clone());

    let device_name = vgactl.get_name();
    cpu.mmu.bus.add_device(DeviceType {
        start: VGACTL_ADDR,
        len: 8,
        instance: vgactl.into(),
        name: device_name,
    });

    // device vga
    let vga = DeviceVGA::new(canvas, vgactl_msg);
    let device_name = vga.get_name();
    cpu.mmu.bus.add_device(DeviceType {
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

    cpu.mmu.bus.add_device(DeviceType {
        start: KBD_ADDR,
        len: 8,
        instance: device_kb.into(),
        name: device_name,
    });
    // device mouse
    let (mouse_sdl_tx, mouse_sdl_rx): (RingSender<DeviceMouseItem>, RingReceiver<DeviceMouseItem>) =
        ring_channel(NonZeroUsize::new(1).unwrap());
    let device_mouse = DeviceMouse::new(mouse_sdl_rx);

    cpu.mmu.bus.add_device(DeviceType {
        start: MOUSE_ADDR,
        len: 16,
        instance: device_mouse.into(),
        name: "Mouse",
    });

    // device sifive_uart
    let (mut sifive_uart_tx, sifive_uart_rx): (RingSender<i32>, RingReceiver<i32>) =
        ring_channel(NonZeroUsize::new(64).unwrap());

    let device_sifive_uart = DeviceSifiveUart::new(sifive_uart_rx);

    cpu.mmu.bus.add_device(DeviceType {
        start: SIFIVE_UART_BASE,
        len: 0x1000,
        instance: device_sifive_uart.into(),
        name: "Sifive_Uart",
    });
    // show device address map
    println!("{0}", cpu.mmu.bus);

    // create another thread to simmulate the cpu_core
    // while the main thread is used to handle sdl events
    // which will be send to coresponding devices through ring_channel
    let cpu_main = thread::spawn(move || {
        // start sim
        cpu.cpu_state = CpuState::Running;
        let mut cycle: u128 = 0;
        while cpu.cpu_state == CpuState::Running {
            cpu.execute(1);
            cycle += 1;
        }
        println!("total:{cycle}");

        // dump signature for riscof
        args.signature
            .map(|x| cpu.dump_signature(&x))
            .unwrap_or_else(|| println!("no signature"));

        // send signal to stop the trace thread
        signal_term_cpucore.store(true, Ordering::Relaxed);
    });

    let trace_thread = thread::spawn(move || {
        while !signal_term_trace.load(Ordering::Relaxed) {
            trace_log.run();
            // std::thread::sleep(Duration::from_millis(100));
        }
    });

    thread::spawn(move || {
        let stdin = io::stdin();
        let mut handle = stdin.lock().bytes();
        while !signal_term_uart.load(Ordering::Relaxed) {
            let x = handle.next();
            if let Some(Ok(ch)) = x {
                sifive_uart_tx.send(ch as i32).unwrap();
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });
    // the main thread to handle sdl events
    handle_sdl_event(cpu_main, event_system, kb_am_tx, kb_sdl_tx, mouse_sdl_tx);

    trace_thread.join().unwrap();
}

fn send_key_event(tx: &mut RingSender<DeviceKbItem>, val: Scancode, keydown: bool) {
    tx.send(DeviceKbItem {
        scancode: val,
        is_keydown: keydown,
    })
    .expect("Key event send error");
}

fn handle_sdl_event(
    cpu_handle: JoinHandle<()>,
    static_event: sdl2::EventSubsystem,
    mut kb_am_tx: RingSender<DeviceKbItem>,
    mut kb_sdl_tx: RingSender<Keycode>,
    mut mouse_sdl_tx: RingSender<DeviceMouseItem>,
) {
    // thread::spawn(move || -> ! {
    let _event_system = static_event;
    let _sdl_context = _event_system.sdl();
    let mut event_pump = _sdl_context.event_pump().expect("fail to get event_pump");
    while !cpu_handle.is_finished() {
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
    // });
}

#[cfg(test)]
mod isa_test {
    use std::{fs, path::Path};

    use crate::{
        bus::DeviceType,
        cpu_core::{CpuCore, CpuState},
        device::device_dram::DeviceDram,
        device::device_trait::{DeviceBase, MEM_BASE},
    };

    fn start_test(img: &str) -> bool {
        let mut cpu = CpuCore::new(None);

        // device dram
        let mut mem = DeviceDram::new(128 * 1024 * 1024);
        mem.load_binary(img);
        let device_name = mem.get_name();
        cpu.mmu.bus.add_device(DeviceType {
            start: MEM_BASE,
            len: mem.capacity as u64,
            instance: mem.into(),
            name: device_name,
        });

        cpu.cpu_state = CpuState::Running;
        let mut cycle: u128 = 0;
        while cpu.cpu_state == CpuState::Running {
            cpu.execute(1);
            cycle += 1;
            cpu.check_to_host();
        }
        println!("total:{cycle}");

        cpu.cpu_state == CpuState::Stop
    }
    const TESTS_PATH: &str = "/home/leesum/workhome/riscv-tests/isa/build/bin";

    struct TestRet {
        pub name: String,
        pub ret: bool,
    }

    #[test]
    fn run_arch_test_onece() {
        // vm_boot:000000008000cdb0
        // handle_fault:0000000000002ae4
        // handle_fault:0000000000002ae4
        // handle_fault:0000000000003000
        // handle_fault:0000000000003000
        // handle_fault:0000000000003000
        let path = "/home/leesum/workhome/riscv-tests/benchmarks/dhrystone.bin";
        let ret = start_test(path);
        println!("{ret}");
    }

    #[test]
    fn run_arch_tests() {
        let sikp_files = vec![
            // "rv64si-p-dirty.bin",
            // "rv64si-p-icache-alias.bin",
            // "rv64mi-p-access.bin",
            // "rv64mi-p-illegal.bin",
            "rv64ui-p-ma_data.bin",
            // "rv64mi-p-breakpoint.bin",
            // "rv64mi-p-zicntr.bin",
            // "rv64mi-p-sbreak.bin",
            // "rv64si-p-sbreak.bin",
            "rv64ui-v-ma_data.bin",
        ];
        let test2_dir = Path::new(TESTS_PATH);
        let mut tests_ret: Vec<TestRet> = Vec::new();
        for entry in fs::read_dir(test2_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            let file_name = path.file_name().unwrap().to_str().unwrap();
            if sikp_files.contains(&file_name) {
                continue;
            }
            if let Some(p) = path.to_str() {
                let ret = start_test(p);
                tests_ret.push(TestRet {
                    name: String::from(file_name),
                    ret,
                });
            }
        }

        tests_ret
            .iter()
            .filter(|item| item.ret)
            .for_each(|x| println!("{:40}{}", x.name, x.ret));
        tests_ret
            .iter()
            .filter(|item| !item.ret)
            .for_each(|x| println!("{:40}{}", x.name, x.ret));

        // tests_ret.iter().for_each(|x| {
        //     assert!(x.ret);
        // });
    }
}
