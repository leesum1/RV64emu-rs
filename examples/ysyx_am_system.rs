// extern crate rv64emu;

// use clap::Parser;
// #[allow(unused_imports)]
// use rv64emu::tools::Fifobounded;
// use rv64emu::{
//     config::Config,
//     device::device_am_vga::VGA_BUF_SIZE,
//     tools::{rc_refcell_new, FifoUnbounded},
// };

// #[allow(unused_imports)]
// use std::{
//     cell::Cell,
//     cell::RefCell,
//     io::{self, Read},
//     num::NonZeroUsize,
//     process,
//     rc::Rc,
//     sync::{
//         atomic::{AtomicBool, Ordering},
//         Arc,
//     },
//     thread,
//     time::Duration,
// };
// use std::{
//     io::{stdin, Write},
//     sync::Mutex,
// };

// use log::{info, LevelFilter};
// use rv64emu::{device::device_16550a::Device16550aUART, rvsim::RVsim};

// use rv64emu::device::{
//     device_am_kb::{DeviceKB, DeviceKbItem},
//     device_am_mouse::{DeviceMouse, DeviceMouseItem},
//     device_am_vga::DeviceVGA,
//     device_am_vgactl::DeviceVGACTL,
//     device_trait::{FB_ADDR, KBD_ADDR, MOUSE_ADDR, VGACTL_ADDR},
// };
// use sdl2::{
//     event::Event,
//     keyboard::{Keycode, Scancode},
// };

// use crate::rv64emu::tools::RcRefCell;

// use crate::{
//     rv64emu::device::{
//         device_am_rtc::DeviceRTC,
//         device_am_uart::DeviceUart,
//         device_memory::DeviceMemory,
//         device_trait::DeviceBase,
//         device_trait::{MEM_BASE, RTC_ADDR, SERIAL_PORT},
//     },
//     rv64emu::rv64core::bus::{Bus, DeviceType},
//     rv64emu::rv64core::cpu_core::CpuCoreBuild,
// };

// // -------------Device Tree MAP-------------
// // name:CLINT           Area:0X02000000-->0X02010000,len:0X00010000
// // name:PLIC            Area:0X0C000000-->0X10000000,len:0X04000000
// // name:DRAM            Area:0X80000000-->0X88000000,len:0X08000000
// // name:UART            Area:0XA00003F8-->0XA00003F9,len:0X00000001
// // name:RTC             Area:0XA0000048-->0XA0000050,len:0X00000008
// // name:VGA_CTL         Area:0XA0000100-->0XA0000108,len:0X00000008
// // name:VGA_FB          Area:0XA1000000-->0XA1075300,len:0X00075300
// // name:KeyBorad_AM     Area:0XA0000060-->0XA0000068,len:0X00000008
// // name:Mouse           Area:0XA0000070-->0XA0000080,len:0X00000010
// // name:Sifive_Uart     Area:0XC0000000-->0XC0001000,len:0X00001000

// #[derive(Parser, Debug)]
// #[command(author, version, about, long_about = None)]
// struct Args {
//     #[arg(long, value_name = "FILE")]
//     /// IMG bin copy to ram
//     img: Option<String>,
//     #[arg(long, value_name = "FILE")]
//     /// IMG bin copy to xipflash
//     xipflash: Option<String>,
//     #[arg(long, value_name = "HEX")]
//     /// the first instruction address,default:0x80000000
//     boot_pc: Option<String>,
//     #[arg(short, long, value_name = "USIZE")]
//     /// Number of harts,default:1
//     num_harts: Option<usize>,
// }

// fn main() {
//     simple_logger::SimpleLogger::new()
//         .with_level(LevelFilter::Debug)
//         .init()
//         .unwrap();
//     let args = Args::parse();

//     if args.img.is_none() && args.xipflash.is_none() {
//         panic!("Please specify the img or xipflash\n");
//     }

//     let ready_to_run = Arc::new(AtomicBool::new(false));

//     let signal_term = Arc::new(AtomicBool::new(false));
//     let signal_term_cpucore = signal_term.clone();
//     #[allow(clippy::redundant_clone)]
//     #[allow(unused_variables)]
//     let signal_term_uart = signal_term.clone();

//     #[allow(unused_variables)]
//     #[allow(clippy::redundant_clone)]
//     let signal_term_sdl_event = signal_term.clone();
//     #[allow(unused_variables)]
//     #[allow(clippy::redundant_clone)]
//     let signal_term_uart = signal_term.clone();

//     let bus_u = rc_refcell_new(Bus::new());

//     // device dram len:0X08000000
//     let mem = DeviceMemory::new(128 * 1024 * 1024);

//     bus_u.borrow_mut().add_device(DeviceType {
//         start: MEM_BASE,
//         len: mem.size() as u64,
//         instance: Box::new(mem),
//         name: "RAM",
//     });

//     // device flash len:0X01000000
//     let falsh = DeviceMemory::new(128 * 1024 * 1024);

//     bus_u.borrow_mut().add_device(DeviceType {
//         start: 0x3000_0000,
//         len: falsh.size() as u64,
//         instance: Box::new(falsh),
//         name: "XIPFLASH",
//     });

//     let uart_tx_fifo = FifoUnbounded::new(crossbeam_queue::SegQueue::<u8>::new());
//     let uart_rx_fifo = FifoUnbounded::new(crossbeam_queue::SegQueue::<u8>::new());

//     let rx_fifo = uart_rx_fifo.clone();
//     let tx_fifo = uart_tx_fifo.clone();

//     thread::spawn(move || loop {
//         let mut buf = [0; 1];
//         if let Ok(n) = stdin().read(&mut buf) {
//             if n > 1 {
//                 panic!("Read {} characters into a 1 byte buffer", n);
//             }
//             if n == 1 {
//                 rx_fifo.push(buf[0]);
//             }
//             // Nothing needs to be sent for n == 0
//         }
//         std::thread::sleep(Duration::from_millis(100));
//     });

//     let uart_tx_thread = thread::spawn(move || loop {
//         while !tx_fifo.is_empty() {
//             if let Some(c) = tx_fifo.pop() {
//                 print!("{}", c as char)
//             }
//         }
//         io::stdout().flush().unwrap();
//         if signal_term_uart.load(Ordering::Relaxed) {
//             break;
//         }
//         std::thread::sleep(Duration::from_millis(50));
//     });

//     // device am_uart
//     let uart = DeviceUart::new(uart_tx_fifo.clone());

//     bus_u.borrow_mut().add_device(DeviceType {
//         start: SERIAL_PORT,
//         len: 1,
//         instance: Box::new(uart),
//         name: "AM_UART",
//     });
//     // device 16650_uart
//     let device_16650_uart = Device16550aUART::new(uart_tx_fifo, uart_rx_fifo);

//     bus_u.borrow_mut().add_device(DeviceType {
//         start: 0x1000_0000,
//         len: 0x1000,
//         instance: Box::new(device_16650_uart),
//         name: "16550a_uart",
//     });

//     // device rtc
//     let rtc = DeviceRTC::new();
//     let device_name = rtc.get_name();

//     bus_u.borrow_mut().add_device(DeviceType {
//         start: RTC_ADDR,
//         len: 8,
//         instance: Box::new(rtc),
//         name: device_name,
//     });

//     let hart_num: usize = args.num_harts.unwrap_or(1);

//     let boot_pc = if let Some(x) = args.boot_pc.as_ref() {
//         u64::from_str_radix(
//             x.trim_start_matches(|c| c == '0' || c == 'x' || c == 'X'),
//             16,
//         )
//         .expect("boot_pc is not a valid hex number")
//     } else {
//         0x8000_0000
//     };

//     info!("boot_pc:0x{:x}", boot_pc);

//     let mut config = Config::new();
//     config.set_tlb_size(256);
//     config.set_icache_size(4096);
//     config.set_decode_cache_size(4096);
//     config.set_mmu_type("bare");
//     config.set_isa("rv64im");

//     let config = Rc::new(config);

//     let mut hart_vec = Vec::new();

//     // create hart according to the number of harts
//     for hart_id in 0..hart_num {
//         let hart = rc_refcell_new(
//             CpuCoreBuild::new(bus_u.clone(), config.clone())
//                 .with_boot_pc(boot_pc)
//                 .with_hart_id(hart_id)
//                 .with_smode(false)
//                 .build(),
//         );
//         hart_vec.push(hart);
//     }

//     let mut sim = RVsim::new(hart_vec, 23456);
//     if let Some(ram_img) = args.img {
//         sim.load_image(&ram_img);
//     }

//     // create another thread to simmulate the harts
//     // while the main thread is used to handle sdl events
//     // which will be send to the coresponding devices through ring_channel
//     let ready_to_run_sim = ready_to_run.clone();
//     let cpu_main = thread::spawn(move || {
//         // wait for all devices to be ready
//         while !ready_to_run_sim.load(Ordering::Relaxed) {
//             std::thread::sleep(Duration::from_millis(100));
//         }

//         sim.run();

//         // send signal to all other threads
//         signal_term_cpucore.store(true, Ordering::Release);
//     });

//     create_sdl2_devices(bus_u, signal_term_sdl_event, ready_to_run);

//     cpu_main.join().unwrap();
//     uart_tx_thread.join().unwrap();
// }

// fn send_key_event(tx: &Fifobounded<DeviceKbItem>, val: Scancode, keydown: bool) {
//     tx.force_push(DeviceKbItem {
//         scancode: val,
//         is_keydown: keydown,
//     });
// }

// fn create_sdl2_devices(
//     bus_u: RcRefCell<Bus>,
//     signal_term_sdl_event: Arc<AtomicBool>,
//     ready_to_run: Arc<AtomicBool>,
// ) {
//     /*--------init sdl --------*/
//     // subsequnt devices are base on sdl2 api
//     // 1. device vga
//     // 2. device kb
//     // 3. device mouse
//     let sdl_context = sdl2::init().unwrap();
//     let video_subsystem = sdl_context.video().unwrap();
//     let event_pump: sdl2::EventPump = sdl_context.event_pump().expect("fail to get event_pump");

//     let window = video_subsystem
//         .window("rust-sdl2 demo: Video", 800, 600)
//         .position_centered()
//         .opengl()
//         .build()
//         .map_err(|e| e.to_string())
//         .unwrap();

//     let mut canvas = window.into_canvas().software().build().expect("canvas err");
//     canvas.set_scale(2.0, 2.0).unwrap();

//     // device vgactl
//     let vgactl_msg = Rc::new(Cell::new(false));

//     let vgactl = DeviceVGACTL::new(vgactl_msg.clone());

//     let device_name = vgactl.get_name();
//     bus_u.borrow_mut().add_device(DeviceType {
//         start: VGACTL_ADDR,
//         len: 8,
//         instance: Box::new(vgactl),
//         name: device_name,
//     });

//     // device vga

//     let vga_fb = Arc::new(Mutex::new(vec![0_u8; VGA_BUF_SIZE].into_boxed_slice()));

//     let vga = DeviceVGA::new(vga_fb.clone());
//     let device_name = vga.get_name();
//     bus_u.borrow_mut().add_device(DeviceType {
//         start: FB_ADDR,
//         len: DeviceVGA::get_size() as u64,
//         instance: Box::new(vga),
//         name: device_name,
//     });

//     let kb_am_fifo = Fifobounded::<DeviceKbItem>::new(crossbeam_queue::ArrayQueue::new(16));

//     let kb_sdl_fifo =
//         Fifobounded::<sdl2::keyboard::Keycode>::new(crossbeam_queue::ArrayQueue::new(16));

//     let device_kb = DeviceKB::new(kb_am_fifo.clone(), kb_sdl_fifo.clone());

//     let device_name = device_kb.get_name();
//     bus_u.borrow_mut().add_device(DeviceType {
//         start: KBD_ADDR,
//         len: 8,
//         instance: Box::new(device_kb),
//         name: device_name,
//     });

//     // device am_mouse
//     let mouse_fifo = Fifobounded::<DeviceMouseItem>::new(crossbeam_queue::ArrayQueue::new(16));

//     let device_mouse = DeviceMouse::new(mouse_fifo.clone());

//     bus_u.borrow_mut().add_device(DeviceType {
//         start: MOUSE_ADDR,
//         len: 16,
//         instance: Box::new(device_mouse),
//         name: "AM_Mouse",
//     });

//     info!("{0}", bus_u.borrow_mut());
//     info!("start sdl event loop");

//     handle_sdl_event(
//         signal_term_sdl_event,
//         event_pump,
//         kb_am_fifo,
//         kb_sdl_fifo,
//         mouse_fifo,
//         vga_fb.clone(),
//         vgactl_msg.clone(),
//         canvas,
//         ready_to_run,
//     );
// }

// fn handle_sdl_event(
//     signal_term: Arc<AtomicBool>,
//     mut event_pump: sdl2::EventPump,
//     kb_am_tx: Fifobounded<DeviceKbItem>,
//     kb_sdl_tx: Fifobounded<Keycode>,
//     mouse_sdl_tx: Fifobounded<DeviceMouseItem>,
//     vga_fb: Arc<Mutex<Box<[u8]>>>,
//     vga_ctl_msg: Rc<Cell<bool>>,
//     mut canvas: sdl2::render::WindowCanvas,
//     ready_to_run: Arc<AtomicBool>,
// ) {
//     let texture_creator = canvas.texture_creator();
//     let mut texture = texture_creator
//         .create_texture_target(sdl2::pixels::PixelFormatEnum::ARGB8888, 400_u32, 300_u32)
//         .map_err(|e| e.to_string())
//         .unwrap();

//     // send signal to sim thread, all devices are ready
//     ready_to_run.store(true, Ordering::Release);
//     while !signal_term.load(Ordering::Relaxed) {
//         let mouse_state = event_pump.mouse_state();

//         mouse_sdl_tx.force_push(DeviceMouseItem {
//             x: (mouse_state.x() / 2) as u32,
//             y: (mouse_state.y() / 2) as u32,
//             mouse_btn_state: mouse_state.to_sdl_state(),
//         });

//         for event in event_pump.poll_iter() {
//             match event {
//                 Event::Quit { .. } => process::exit(0),
//                 Event::KeyUp {
//                     scancode: Some(val),
//                     ..
//                 } => send_key_event(&kb_am_tx, val, false),

//                 Event::KeyDown {
//                     scancode: Some(val),
//                     keycode: Some(sdl_key_code),
//                     ..
//                 } => {
//                     send_key_event(&kb_am_tx, val, true);
//                     kb_sdl_tx.force_push(sdl_key_code);
//                 }

//                 _ => (),
//             }
//         }

//         if vga_ctl_msg.get() {
//             vga_ctl_msg.set(false);
//             let fb = vga_fb.lock().unwrap();
//             let fb_copy = fb.clone();
//             drop(fb);
//             texture
//                 .update(None, &fb_copy, 4 * 400)
//                 .expect("update texture failed");
//             canvas.copy(&texture, None, None).unwrap();
//             canvas.present();
//         }
//         std::thread::sleep(Duration::from_millis(20));
//     }
// }


