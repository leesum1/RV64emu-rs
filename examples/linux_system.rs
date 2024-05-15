extern crate rv64emu;

use clap::Parser;
#[allow(unused_imports)]
use rv64emu::tools::Fifobounded;
use rv64emu::{
    config::Config,
    tools::{rc_refcell_new, FifoUnbounded},
};

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
    fs,
    io::{stdin, Write},
};

use log::{info, LevelFilter};
use rv64emu::{device::device_16550a::Device16550aUART, rvsim::RVsim};

use crate::{
    rv64emu::device::{
        device_memory::DeviceMemory, device_sifive_plic::SIFIVE_UART_IRQ,
        device_sifive_uart::DeviceSifiveUart, device_trait::MEM_BASE,
    },
    rv64emu::rv64core::bus::{Bus, DeviceType},
    rv64emu::rv64core::cpu_core::CpuCoreBuild,
};

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
}
// -------------Device Tree MAP-------------
// name:CLINT           Area:0X02000000-->0X02010000,len:0X00010000
// name:PLIC            Area:0X0C000000-->0X10000000,len:0X04000000
// name:RAM             Area:0X80000000-->0X88000000,len:0X08000000
// name:XIPFLASH        Area:0X30000000-->0X38000000,len:0X08000000
// name:16550a_uart     Area:0X10000000-->0X10001000,len:0X00001000
// name:Sifive_Uart     Area:0XC0000000-->0XC0001000,len:0X00001000
fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .init()
        .unwrap();

    let args = Args::parse();

    if args.img.is_none() && args.xipflash.is_none() {
        panic!("Please specify the img or xipflash");
    }

    // config
    let mut config = Config::new();
    config.set_tlb_size(256);
    config.set_icache_size(4096);
    config.set_decode_cache_size(4096);
    config.set_mmu_type("sv39"); // sv39 sv48 sv57
    config.set_isa("rv64imac");
    config.set_s_mode();
    let config = Rc::new(config);

    let signal_term = Arc::new(AtomicBool::new(false));

    let bus_u = rc_refcell_new(Bus::new());

    // device dram len:0X08000000
    let mem = DeviceMemory::new(0x8000000);

    bus_u.borrow_mut().add_device(DeviceType {
        start: MEM_BASE,
        len: mem.size() as u64,
        instance: Box::new(mem),
        name: "RAM",
    });

    // device flash len:0X08000000
    let mut flash = DeviceMemory::new(0x8000000);

    if let Some(xipflash) = args.xipflash {
        let flash_data = fs::read(xipflash).unwrap();
        flash.load_binary(&flash_data);
    }
    bus_u.borrow_mut().add_device(DeviceType {
        start: 0x3000_0000,
        len: flash.size() as u64,
        instance: Box::new(flash),
        name: "XIPFLASH",
    });

    // we use crossbeam_queue::SegQueue as the uart fifo
    // each uart has a tx and rx fifo, and we use them to communicate with the host pc
    // at the host side, we use two threads to handle the uart tx and rx
    let uart_tx_fifo = FifoUnbounded::new(crossbeam_queue::SegQueue::<u8>::new());
    let uart_rx_fifo = FifoUnbounded::new(crossbeam_queue::SegQueue::<u8>::new());

    let rx_fifo = uart_rx_fifo.clone();
    let tx_fifo = uart_tx_fifo.clone();
    let signal_term_uart = signal_term.clone();
    thread::spawn(move || loop {
        let mut buf = [0; 1];
        if let Ok(n) = stdin().read(&mut buf) {
            if n > 1 {
                panic!("Read {} characters into a 1 byte buffer", n);
            }
            if n == 1 {
                rx_fifo.push(buf[0]);
            }
            // Nothing needs to be sent for n == 0
        }
        std::thread::sleep(Duration::from_millis(100));
    });

    let uart_tx_thread = thread::spawn(move || loop {
        while !tx_fifo.is_empty() {
            if let Some(c) = tx_fifo.pop() {
                print!("{}", c as char)
            }
        }
        io::stdout().flush().unwrap();
        if signal_term_uart.load(Ordering::Relaxed) {
            break;
        }
        // std::thread::sleep(Duration::from_millis(100));
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

    // sifive_uart support irq
    bus_u
        .borrow_mut()
        .plic
        .instance
        .register_irq_source(SIFIVE_UART_IRQ, Rc::clone(&device_sifive_uart.irq_pending));

    bus_u.borrow_mut().add_device(DeviceType {
        start: 0xc0000000,
        len: 0x1000,
        instance: Box::new(device_sifive_uart),
        name: "Sifive_Uart",
    });

    let boot_pc = args.boot_pc.as_ref().map_or(0x8000_0000, |x| {
        let cleaned = x.trim_start_matches(|c| c == '0' || c == 'x' || c == 'X');
        u64::from_str_radix(cleaned, 16)
            .unwrap_or_else(|_| panic!("boot_pc is not a valid hex number"))
    });

    // show device address map
    info!("{0}", bus_u.borrow_mut());
    info!("boot_pc:0x{:x}", boot_pc);

    let hart_num: usize = args.num_harts.unwrap_or(1);
    let mut hart_vec = Vec::new();
    // create harts
    for hart_id in 0..hart_num {
        let hart = rc_refcell_new(
            CpuCoreBuild::new(bus_u.clone(), config.clone())
                .with_boot_pc(boot_pc)
                .with_hart_id(hart_id)
                .with_smode(true)
                .build(),
        );
        hart_vec.push(hart);
    }

    // create another thread to simmulate the harts
    // let cpu_main = thread::spawn(move || {
    let mut sim = RVsim::new(hart_vec, 23456);
    if let Some(ram_img) = args.img {
        sim.load_image(&ram_img);
    }

    sim.run();
    // notify the uart thread to exit
    signal_term.store(true, Ordering::Relaxed);
    // });

    // cpu_main.join().unwrap();
    uart_tx_thread.join().unwrap();
}
