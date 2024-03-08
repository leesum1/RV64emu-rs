use core::ops;

#[cfg(feature = "std")]
use std::{fs::File, io::Write};

use alloc::{
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};
use elf::{
    abi::{EM_RISCV, PT_LOAD},
    endian::AnyEndian,
};
use log::info;

use crate::{
    config::Config,
    dbg::{debug_module_new::DebugModule, jtag_driver::JtagDriver, remote_bitbang::RemoteBitBang},
};
#[allow(unused_imports)]
use crate::{
    rv64core::{
        bus::Bus,
        cpu_core::{CpuCore, CpuState},
        // csr_regs_define::Misa,
        inst::inst_base::FesvrCmd,
    },
    tools::RcRefCell,
};

// #[derive(Default)]
pub struct RVsim {
    /* riscv-arch-tests */
    tohost: Option<u64>,
    _fromhost: Option<u64>,
    /* riscof tests */
    signature_range: Option<ops::Range<u64>>,
    signature_file: Option<String>,
    bus: RcRefCell<Bus>,
    pub harts: Vec<RcRefCell<CpuCore>>,
    // name: String,value: u64
    elf_symbols: hashbrown::HashMap<String, u64>,

    // debug
    remote_bitbang: RemoteBitBang,
    jtag_driver: JtagDriver,
    // Config
    config: Rc<Config>,
}

impl RVsim {
    pub fn new(harts: Vec<RcRefCell<CpuCore>>) -> Self {
        let bus = harts[0].borrow_mut().mmu.caches.borrow_mut().bus.clone();

        let dm = DebugModule::new(harts[0].clone());
        let remote_bitbang = RemoteBitBang::new("0.0.0.0", 23456);
        let mut jtag_driver = JtagDriver::new(dm);

        Self {
            harts,
            bus,
            config: Rc::new(Config::new()),
            elf_symbols: hashbrown::HashMap::new(),
            tohost: None,
            _fromhost: None,
            signature_range: None,
            signature_file: None,
            remote_bitbang,
            jtag_driver,
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
    fn _load_elf(&mut self, slice: &[u8], collect_symbol: bool) {
        let elf_data = elf::ElfBytes::<AnyEndian>::minimal_parse(slice);
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
                let mut bus = self.bus.borrow_mut();

                bus.copy_from_slice(p.p_paddr, data).unwrap();
            });
            info!("Elf file match,elf load success");

            // Collect elf symbols into self.elf_symbols(hashmap)
            if collect_symbol {
                self.collect_elf_symbols(&elf_data);
            }
        } else {
            let boot_pc = self.harts.get(0).unwrap().borrow().pc;

            let mut bus = self.bus.borrow_mut();
            bus.copy_from_slice(boot_pc, slice).unwrap();

            info!("Elf file not match, bin load success");
        }
    }

    #[cfg(feature = "std")]
    pub fn load_image(&mut self, file_name: &str) {
        let file_data = std::fs::read(file_name).unwrap();
        info!("load image from file: {}", file_name);
        self._load_elf(&file_data, true);
    }
    pub fn load_image_from_slice(&mut self, slice: &[u8]) {
        self._load_elf(slice, false);
    }

    pub fn prepare_to_run(&mut self) {
        self.harts
            .iter_mut()
            .for_each(|hart| hart.borrow_mut().cpu_state = CpuState::Running);
    }

    // run 5000 cycles
    pub fn run_once(&mut self) {
        self.remote_bitbang.tick(&mut self.jtag_driver);

        self.harts.iter_mut().for_each(|hart| {
            hart.borrow_mut().execute(5000);
        });
        let mut bus = self.bus.borrow_mut();
        bus.update();
        bus.clint.instance.tick(5000 / 10);
        bus.plic.instance.tick();
        drop(bus);

        // #[cfg(feature = "std")]
        // self.check_to_host();
    }

    pub fn step(&mut self) {
        self.harts.iter_mut().for_each(|hart| {
            hart.borrow_mut().execute(1);
        });
        let mut bus = self.bus.borrow_mut();
        bus.update();
        bus.clint.instance.tick(1);
        bus.plic.instance.tick();
        drop(bus);
    }

    // true: exit, false: abort
    pub fn is_finish(&self) -> bool {
        self.harts
            .iter()
            .any(|hart| hart.borrow().cpu_state == CpuState::Stop)
    }

    pub fn is_exit_normal(&self) -> bool {
        self.harts
            .iter()
            .all(|hart| hart.borrow().cpu_state == CpuState::Stop)
    }

    pub fn show_perf(&self) {
        self.harts.iter().for_each(|hart| {
            hart.borrow().show_perf();
        });
    }

    // true: exit, false: abort
    pub fn run(&mut self) -> bool {
        self.prepare_to_run();

        while !self.is_finish() {
            self.run_once();
        }
        #[cfg(feature = "std")]
        self.dump_signature();
        self.show_perf();
        self.is_exit_normal()
    }

    // for riscv-tests
    // It seems in riscv-tests ends with end code
    // written to a certain physical memory address
    // (0x80001000 in mose test cases)
    #[cfg(feature = "std")]
    pub fn check_to_host(&mut self) {
        if self.tohost.is_none() {
            return;
        }

        self.harts[0].borrow_mut().cache_system.borrow_mut().clear();
        let mut bus_u = self.bus.borrow_mut();
        let tohost = self.tohost.unwrap_or(0x8000_1000);
        let data = bus_u.read(tohost, 8).unwrap();
        // !! must clear mem after read
        bus_u.write(tohost, 0, 8).unwrap();
        // debug!("check to host: {:#x}", data);
        let cmd = FesvrCmd::from(data);
        if let Some(pass) = cmd.syscall_device() {
            if pass {
                self.harts
                    .iter_mut()
                    .for_each(|hart| hart.borrow_mut().cpu_state = CpuState::Stop);
            }
            // fail
            else {
                self.harts
                    .iter_mut()
                    .for_each(|hart| hart.borrow_mut().cpu_state = CpuState::Abort);
                info!("FAIL WITH EXIT CODE:{}", cmd.exit_code())
            }
        }
        cmd.character_device_write();
    }

    pub fn set_signature_file(&mut self, file_name: String) {
        self.signature_file = Some(file_name);
    }

    // for riscof
    #[cfg(feature = "std")]
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
            |err| info!("{err}"),
            |mut file| {
                let mut bus_u = self.bus.borrow_mut();
                for i in sig_range.step_by(4) {
                    let tmp_data = bus_u.read(i, 4).unwrap();
                    file.write_fmt(format_args! {"{tmp_data:08x}\n"}).unwrap();
                }
                info!("dump signature done, file: {}", file_name);
            },
        );
    }
}
