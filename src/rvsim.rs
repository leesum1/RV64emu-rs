use std::{fs::File, io::Write, ops};

use elf::{
    abi::{EM_RISCV, PT_LOAD},
    endian::AnyEndian,
};
use log::{info, warn};

use crate::rv64core::{
    bus::Bus,
    cpu_core::{CpuCore, CpuState},
    // csr_regs_define::Misa,
    inst::inst_base::FesvrCmd, traptype::RVmutex,
};

#[derive(Default)]
pub struct RVsim {
    /* riscv-arch-tests */
    tohost: Option<u64>,
    _fromhost: Option<u64>,
    /* riscof tests */
    signature_range: Option<ops::Range<u64>>,
    signature_file: Option<String>,
    bus: RVmutex<Bus>,
    harts: Vec<CpuCore>,
    // name: String,value: u64
    elf_symbols: hashbrown::HashMap<String, u64>,
}

impl RVsim {
    pub fn new(harts: Vec<CpuCore>) -> Self {
        let bus = harts[0].mmu.caches.lock().bus.clone();
        Self {
            harts,
            bus,
            ..Default::default()
        }
    }
    fn get_symbol_values(&mut self) {
        let tohost_addr = self.elf_symbols.get("tohost").copied();
        let fromhost_addr = self.elf_symbols.get("fromhost").copied();
        let begin_regstate_addr: Option<u64> = self.elf_symbols.get("begin_signature").copied();
        let end_regstate_addr = self.elf_symbols.get("end_signature").copied();

        self.tohost = tohost_addr;
        self._fromhost = fromhost_addr;
        if let (Some(begin_regstate_addr), Some(end_regstate_addr)) =
            (begin_regstate_addr, end_regstate_addr)
        {
            self.signature_range = Some(begin_regstate_addr..end_regstate_addr);
        }

        /* log */
        if let Some(tohost) = self.tohost {
            info!("tohost: {:#x}", tohost);
        }
        if let Some(fromhost) = self._fromhost {
            info!("fromhost: {:#x}", fromhost);
        }
        if let Some(signature_range) = &self.signature_range {
            info!(
                "signature_range: {:#x}..{:#x}",
                signature_range.start, signature_range.end
            );
        }
    }

    fn collect_elf_symbols(&mut self, elf_data: &elf::ElfBytes<AnyEndian>) {
        let common_data = elf_data.find_common_data().unwrap();
        if let (Some(symtab), Some(symtab_strs)) = (common_data.symtab, common_data.symtab_strs) {
            for sym in symtab.iter() {
                if let Ok(name) = symtab_strs.get(sym.st_name as usize) {
                    self.elf_symbols.insert(name.to_string(), sym.st_value);
                    // debug!("elf symbol: {} = {:#x}", name, sym.st_value);
                }
            }
            info!("collected elf symbols: {}", self.elf_symbols.len());
            // get needed symbols value
            self.get_symbol_values();
        }
    }

    pub fn load_elf(&mut self, file_name: &str) {
        let file_data = std::fs::read(file_name).unwrap();

        let elf_data = elf::ElfBytes::<AnyEndian>::minimal_parse(&file_data);

        if let Ok(elf_data) = elf_data {
            let ehdr: elf::file::FileHeader<AnyEndian> = elf_data.ehdr;
            // Check e_machine
            assert_eq!(ehdr.e_machine, EM_RISCV);
            // Check Program header Table
            assert_ne!(ehdr.e_phnum, 0);
            let phdr: elf::parse::ParsingTable<AnyEndian, elf::segment::ProgramHeader> =
                elf_data.segments().unwrap();

            // Load program segments to memory
            phdr.iter().filter(|x| x.p_type == PT_LOAD).for_each(|p| {
                let data = elf_data.segment_data(&p).unwrap();
                assert_eq!(data.len(), p.p_filesz as usize);
                let mut bus = self.bus.lock();
                // todo! write 8 bytes at a time
                for addr in (p.p_paddr)..(p.p_paddr + p.p_filesz) {
                    bus.write(addr, data[(addr - p.p_paddr) as usize].into(), 1)
                        .unwrap();
                }
            });
            info!("Elf file match,elf load success:{}", file_name);

            // Collect elf symbols into self.elf_symbols(hashmap)
            self.collect_elf_symbols(&elf_data);
        } else {
            let boot_pc = self.harts.get(0).unwrap().pc;
            let mut bus = self.bus.lock();

            // todo! write 8 bytes at a time
            for (i, data) in file_data.iter().enumerate() {
                bus.write(boot_pc + i as u64, (*data).into(), 1).unwrap();
            }
            info!("Elf file not match, bin load success:{}", file_name);
        }
    }

    // true: exit, false: abort
    pub fn run(&mut self) -> bool {
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
                let mut bus = self.bus.lock();
                bus.update();
                // bus.clint.instance.tick(5000 / 100);
                bus.clint.instance.tick(500);
                bus.plic.instance.tick();
            });
            self.check_to_host();
        }
        self.dump_signature();

        self.harts.iter().all(|hart| {
            hart.cache_system.lock().show_perf();
            hart.cpu_state != CpuState::Abort
        })
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

        self.harts[0].cache_system.lock().clear();
        let mut bus_u = self.bus.lock();
        let tohost = self.tohost.unwrap_or(0x8000_1000);
        let data = bus_u.read(tohost, 8).unwrap();
        // !! must clear mem
        bus_u.write(tohost, 0, 8).unwrap();
        // debug!("check to host: {:#x}", data);
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
        if self.signature_file.is_none() {
            return;
        }
        // todo! how to remove this clone?
        let file_name: String = self.signature_file.clone().unwrap();
        let sig_range = self.signature_range.clone().unwrap();

        let fd: Result<File, std::io::Error> = File::create(&file_name);
        info!(
            "sig_start: {:#x},sig_end: {:#x}",
            sig_range.start, sig_range.end
        );
        fd.map_or_else(
            |err| warn!("{err}"),
            |mut file| {
                let mut bus_u = self.bus.lock();
                for i in sig_range.step_by(4) {
                    let tmp_data = bus_u.read(i, 4).unwrap();
                    file.write_fmt(format_args! {"{tmp_data:08x}\n"}).unwrap();
                }
                info!("dump signature done, file: {}", file_name);
            },
        );
    }
}

// pub struct HartConfig {
//     isa_futrue: Misa,
//     mmu_futrue: u32,
//     mode: u32,
//     boot_pc: u64,
// }
