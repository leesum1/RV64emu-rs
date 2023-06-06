extern crate riscv64_emu;
use std::{fs, path::Path, rc::Rc, sync::Mutex};

use crate::{
    riscv64_emu::device::device_dram::DeviceDram,
    riscv64_emu::device::device_trait::{DeviceBase, MEM_BASE},
    riscv64_emu::rv64core::bus::{Bus, DeviceType},
    riscv64_emu::rv64core::cpu_core::{CpuCoreBuild, CpuState},
};

fn get_riscv_tests_path() -> std::path::PathBuf {
    let root_dir = env!("CARGO_MANIFEST_DIR");
    let bin_path: std::path::PathBuf = Path::new(root_dir)
        .join("ready_to_run")
        .join("riscv-tests")
        .join("bin");
    bin_path
}

fn start_test(img: &str) -> bool {
    let bus_u = Rc::new(Mutex::new(Bus::new()));

    let mut cpu = CpuCoreBuild::new(bus_u.clone())
        .with_boot_pc(0x8000_0000)
        .with_hart_id(0)
        .with_smode(true)
        .build();

    // device dram
    let mut mem: DeviceDram = DeviceDram::new(128 * 1024 * 1024);
    mem.load_binary(img);
    let device_name = mem.get_name();
    bus_u.lock().unwrap().add_device(DeviceType {
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

    // check result
    cpu.cpu_state == CpuState::Stop
}

struct TestRet {
    pub name: String,
    pub ret: bool,
}

#[test]
fn run_arch_tests() {
    // not support misaligned load/store, so skip these tests
    let sikp_files = vec![
        "rv64ui-p-ma_data.bin",
        "rv64ui-v-ma_data.bin",
        "rv64uc-p-rvc.bin", // tohost is 0x8000_3000
                            // "rv64uc-v-rvc.bin",
    ];
    let tests_dir = get_riscv_tests_path();
    let mut tests_ret: Vec<TestRet> = Vec::new();

    for entry in fs::read_dir(tests_dir).unwrap() {
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

    tests_ret.iter().for_each(|x| {
        assert!(x.ret);
    });
}
