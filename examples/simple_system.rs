use std::{env, path::PathBuf};
extern crate rv64emu;

use rv64emu::{
    device::{
        device_am_uart::DeviceUart,
        device_memory::DeviceMemory,
        device_trait::{DeviceBase, MEM_BASE, SERIAL_PORT},
    },
    rv64core::{
        bus::{Bus, DeviceType},
        cpu_core::CpuCoreBuild,
    },
    rvsim::RVsim,
    tools::{fifo_unbounded_new, RcRefCell, rc_refcell_new},
};

fn main() {
    // find binary file path
    let root_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let bin_path = PathBuf::from(&root_dir)
        .join("ready_to_run")
        .join("hello.bin");
    println!("Binary file path: {}", bin_path.display());

    // create system bus, which functions are as follows
    // 1. manage all devices,including plic,clint,and sram
    // 2. shared by all harts
    let bus_u = rc_refcell_new(Bus::new());

    // create hart0 with smode support, some additional features are as follows
    // 1. the first instruction is executed at 0x8000_0000
    // 2. hart0 id is 0
    // 3. smode is enabled
    let hart0 = CpuCoreBuild::new(bus_u.clone())
        .with_boot_pc(0x8000_0000)
        .with_hart_id(0)
        .with_smode(true)
        .build();

    // create device dram with 128MB capacity
    // Mount the dram under the bus
    let mem: DeviceMemory = DeviceMemory::new(128 * 1024 * 1024);

    let device_name = mem.get_name();
    bus_u.borrow_mut().add_device(DeviceType {
        start: MEM_BASE,
        len: mem.size() as u64,
        instance: Box::new(mem),
        name: device_name,
    });

    let uart0_tx_fifo = fifo_unbounded_new::<u8>();

    // device uart
    let uart = DeviceUart::new(uart0_tx_fifo.clone());
    let device_name = uart.get_name();
    bus_u.borrow_mut().add_device(DeviceType {
        start: SERIAL_PORT,
        len: 1,
        instance: Box::new(uart),
        name: device_name,
    });

    // print bus device map
    println!("{0}", bus_u.borrow());

    let harts = vec![hart0];
    let mut sim = RVsim::new(harts);

    // run simulation
    let bin_data = std::fs::read(bin_path).unwrap();
    sim.load_image_from_slice(&bin_data);
    sim.prepare_to_run();
    while !sim.is_finish() {
        sim.run_once();

        while let Some(c) = uart0_tx_fifo.pop() {
            print!("{}", c as char);
        }
    }
}
