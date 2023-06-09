use std::{fs::File, io::Write, ops, rc::Rc, sync::Mutex};

use elf::{
    abi::{EM_RISCV, PT_LOAD},
    endian::AnyEndian,
};
use log::{debug, info, warn};
use riscv64_emu::rv64core::{
    bus::Bus,
    cpu_core::{CpuCore, CpuState},
    inst::inst_base::FesvrCmd,
};

#[derive(Default)]
pub struct RVsim {
    /* riscv-arch-tests */
    tohost: Option<u64>,
    _fromhost: Option<u64>,
    /* riscof tests */
    signature_range: Option<ops::Range<u64>>,
    signature_file: Option<String>,
    bus: Rc<Mutex<Bus>>,
    harts: Vec<CpuCore>,
    // value: u64,name: String
    _elf_symbols: hashbrown::HashMap<u64, String>,
}

impl RVsim {
    pub fn new(harts: Vec<CpuCore>) -> Self {
        let bus = harts[0].mmu.bus.clone();
        Self {
            harts,
            bus,
            ..Default::default()
        }
    }

    pub fn load_elf(&mut self, file_name: &str) {
        let file_data = std::fs::read(file_name).unwrap();

        let elf_data = elf::ElfBytes::<AnyEndian>::minimal_parse(&file_data);

        if let Ok(elf_data) = elf_data {
            let ehdr: elf::file::FileHeader<AnyEndian> = elf_data.ehdr;
            // Check e_machine
            assert_eq!(ehdr.e_machine, EM_RISCV);
            /* Check Program header Table */
            assert_ne!(ehdr.e_phnum, 0);

            let phdr = elf_data.segments().unwrap();

            phdr.iter().filter(|x| x.p_type == PT_LOAD).for_each(|p| {
                let data = elf_data.segment_data(&p).unwrap();
                assert_eq!(data.len(), p.p_filesz as usize);
                let mut bus = self.bus.lock().unwrap();
                // todo! write 8 bytes at a time
                for addr in (p.p_paddr)..(p.p_paddr + p.p_filesz) {
                    bus.write(addr, data[(addr - p.p_paddr) as usize].into(), 1)
                        .unwrap();
                }
            });
            info!("elf load success:{}", file_name);
        } else {
            let boot_pc = self.harts.get(0).unwrap().pc;
            let mut bus = self.bus.lock().unwrap();
            // todo! write 8 bytes at a time

            for (i, data) in file_data.iter().enumerate() {
                bus.write(boot_pc + i as u64, (*data).into(), 1).unwrap();
            }
            info!("bin load success:{}", file_name);
        }
    }
    pub fn run(&mut self) {
        self.harts
            .iter_mut()
            .for_each(|hart| hart.cpu_state = CpuState::Running);

        while self
            .harts
            .iter()
            .all(|hart| hart.cpu_state == CpuState::Running)
        {
            self.harts.iter_mut().for_each(|hart| {
                hart.execute(5000);
                let mut bus = self.bus.lock().unwrap();
                bus.update();
                // bus.clint.instance.tick(5000 / 100);
                bus.clint.instance.tick(500);
                bus.plic.instance.tick();
            });
            self.check_to_host();
        }
    }
    // for riscv-tests
    // It seems in riscv-tests ends with end code
    // written to a certain physical memory address
    // (0x80001000 in mose test cases) so checking
    // the data in the address and terminating the test
    // if non-zero data is written.
    // End code 1 seems to mean pass.
    pub fn check_to_host(&mut self) {
        if self.tohost.is_none() {
            return;
        }
        let mut bus_u = self.bus.lock().unwrap();
        let tohost = self.tohost.unwrap_or(0x8000_1000);
        let data = bus_u.read(tohost, 8).unwrap();
        // !! must clear mem
        bus_u.write(tohost, 0, 8).unwrap();
        debug!("check to host: {:#x}", data);
        let cmd = FesvrCmd::from(data);
        if let Some(pass) = cmd.syscall_device() {
            if pass {
                self.harts
                    .iter_mut()
                    .for_each(|hart| hart.cpu_state = CpuState::Stop);
            }
            // fail
            else {
                self.harts
                    .iter_mut()
                    .for_each(|hart| hart.cpu_state = CpuState::Abort);
                warn!("FAIL WITH EXIT CODE:{}", cmd.exit_code())
            }
        }
        cmd.character_device_write();
    }

    pub fn set_signature_file(&mut self, file_name: String) {
        self.signature_file = Some(file_name);
    }

    // for riscof
    pub fn dump_signature(&mut self) {
        if self.signature_range.is_none() || self.signature_file.is_none() {
            return;
        }
        // todo! how to remove this clone?
        let file_name = self.signature_file.clone().unwrap();
        let sig_range = self.signature_range.clone().unwrap();

        let fd: Result<File, std::io::Error> = File::create(&file_name);
        info!(
            "sig_start: {:#x},sig_end: {:#x}",
            sig_range.start, sig_range.end
        );
        fd.map_or_else(
            |err| warn!("{err}"),
            |mut file| {
                let mut bus_u = self.bus.lock().unwrap();
                for i in sig_range.step_by(4) {
                    let tmp_data = bus_u.read(i, 4).unwrap();
                    file.write_fmt(format_args! {"{tmp_data:08x}\n"}).unwrap();
                }
                info!("dump signature done, file: {}", file_name);
            },
        );
    }
}
