mod bus;
mod cpu_core;
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
    dram::Dram,
};

/// Simple program to greet a person
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

    println!("{}", args.img);

    let mut cpu = CpuCore::new();

    let mut mem = Box::new(Dram::new(128 * 1024 * 1024));
    mem.load_binary(&args.img);

    cpu.bus.add_device(DeviceType {
        start: 0x8000_0000,
        len: mem.capacity as u64,
        instance: mem,
    });

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

    args.signature
        .map(|x| cpu.dump_signature(&x))
        .unwrap_or_else(|| println!("no signature"));

    // let a0_val = cpu.gpr.read_by_name("a0");
    
}
