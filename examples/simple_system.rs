use std::{
    env,
    path::PathBuf,
    sync::Mutex, rc::Rc,
};
extern crate riscv64_emu;

use riscv64_emu::{
    device::{
        device_dram::DeviceDram,
        device_trait::{DeviceBase, MEM_BASE, SERIAL_PORT},
        device_uart::DeviceUart,
    },
    rv64core::{
        bus::{Bus, DeviceType},
        cpu_core::{CpuCoreBuild, CpuState},
    },
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
    let bus_u = Rc::new(Mutex::new(Bus::new()));

    // create hart0 with smode support, some additional features are as follows
    // 1. the first instruction is executed at 0x8000_0000
    // 2. hart0 id is 0
    // 3. smode is enabled
    let mut hart0 = CpuCoreBuild::new(bus_u.clone())
        .with_boot_pc(0x8000_0000)
        .with_hart_id(0)
        .with_smode(true)
        .build();

    // create device dram with 128MB capacity
    // load binary file to dram start address (0x8000_0000)
    // Mount the dram under the bus
    let mut mem: DeviceDram = DeviceDram::new(128 * 1024 * 1024);
    mem.load_binary(bin_path.to_str().unwrap());
    let device_name = mem.get_name();
    bus_u.lock().unwrap().add_device(DeviceType {
        start: MEM_BASE,
        len: mem.capacity as u64,
        instance: mem.into(),
        name: device_name,
    });

    // device uart
    let uart = DeviceUart::new();
    let device_name = uart.get_name();
    bus_u.lock().unwrap().add_device(DeviceType {
        start: SERIAL_PORT,
        len: 1,
        instance: uart.into(),
        name: device_name,
    });

    // print bus device map
    println!("{0}", bus_u.lock().unwrap());

    hart0.cpu_state = CpuState::Running;
    let mut cycle: u64 = 0;
    while hart0.cpu_state == CpuState::Running {
        hart0.execute(1);

        // update clint and plic every 128 cycles
        if cycle % 128 == 0 {
            bus_u.lock().unwrap().update();
            bus_u.lock().unwrap().clint.instance.tick(128);
            bus_u.lock().unwrap().plic.instance.tick();
        }
        cycle += 1;
    }
    println!("total:{cycle}");
}
