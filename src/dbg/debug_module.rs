use core::cell::RefCell;

use alloc::rc::Rc;
use log::{debug, trace};

use super::{
    debug_module_register::{self, *},
    dm_interface::DebugModuleSlave,
};

struct DebugModuleConfig {
    progbuf_count: u32,
    abstract_data_count: u32,
    dm_base: u64,
}

impl DebugModuleConfig {
    pub fn new() -> DebugModuleConfig {
        DebugModuleConfig {
            progbuf_count: 16,
            abstract_data_count: 6,
            dm_base: 0x0,
        }
    }
}

pub struct DebugModule {
    hart0: Rc<RefCell<dyn DebugModuleSlave>>,

    // debug module registers
    progbuf: Box<[u32]>,
    abstract_data: Box<[u32]>,
    dmcontrol: DMControl,
    dmstatus: DMStatus,
    hartinfo: HartInfo,
    abstractcs: Abstractcs,
    command: Command,

    config: DebugModuleConfig,
}

impl DebugModule {
    pub fn new(hart: Rc<RefCell<dyn DebugModuleSlave>>) -> DebugModule {
        let config = DebugModuleConfig::new();

        let dmcontrol = DMControl::new();
        let dmstatus = DMStatus::new()
            .with_version(debug_module_register::debug_const::DMSTATUS_VERSION0_13 as u8)
            .with_authenticated(true);
        let hartinfo = HartInfo::new();
        let abstractcs = Abstractcs::new()
            .with_progbufsize(0)
            .with_datacount(config.abstract_data_count as u8);
        let command = Command::new();

        let mut gprs = Box::new([0; 32]);

        gprs.iter_mut().enumerate().for_each(|(i, x)| {
            *x = i as u64;
        });

        DebugModule {
            hart0: hart,
            progbuf: vec![0; config.progbuf_count as usize].into_boxed_slice(),
            abstract_data: vec![0; config.abstract_data_count as usize].into_boxed_slice(),
            dmcontrol,
            dmstatus,
            hartinfo,
            abstractcs,
            command,
            config,
        }
    }

    pub fn dmi_read(&mut self, address: u64) -> Option<u64> {
        let addr = (address - self.config.dm_base) as usize;

        let abstract_data_range = core::ops::Range {
            start: ABSTRACT_DATA_BASE,
            end: ABSTRACT_DATA_BASE + self.config.abstract_data_count as usize,
        };
        let progbuf_range = core::ops::Range {
            start: PROGBUF_BASE,
            end: PROGBUF_BASE + self.config.progbuf_count as usize,
        };

        let rdata = if abstract_data_range.contains(&addr) {
            let offset = addr - abstract_data_range.start;
            let value = self.abstract_data.get(offset).unwrap_or_else(|| {
                debug!("abstract_data out of range: {:x}", offset);
                &0
            });
            *value as u64
        } else if progbuf_range.contains(&addr) {
            let offset = addr - progbuf_range.start;
            let value = self.progbuf.get(offset).unwrap_or_else(|| {
                debug!("program buffer out of range: {:x}", offset);
                &0
            });
            *value as u64
        } else {
            match addr {
                DMCONTROL_ADDR => {
                    let mut r_dmcontrol = self.dmcontrol;

                    // r_dmcontrol.set_resumereq(self.hart_debug_state.resumeack);

                    let mut hart0 = self.hart0.borrow_mut(); // only support single hart now

                    r_dmcontrol.set_hartreset(hart0.havereset());
                    // only support single hart now
                    r_dmcontrol.set_hasel(false);
                    r_dmcontrol.set_hartselhi(0);
                    r_dmcontrol.set_hartsello(0);

                    r_dmcontrol.set_dmactive(self.dmcontrol.dmactive());
                    r_dmcontrol.set_ndmreset(self.dmcontrol.ndmreset());

                    u32::from(r_dmcontrol) as u64
                }
                DMSTATUS_ADDR => {
                    let mut hart0 = self.hart0.borrow_mut(); // only support single hart now

                    // only support single hart now
                    self.dmstatus.set_anyunavail(false); // TODO:  stickyunavail
                    self.dmstatus.set_allunavail(false);
                    self.dmstatus.set_anynonexistent(false);
                    self.dmstatus.set_allnonexistent(false);

                    // if hart0.havereset() {
                    //     self.dmstatus.set_anyhavereset(true);
                    //     self.dmstatus.set_allhavereset(true);
                    // }

                    self.dmstatus.set_anyhavereset(hart0.havereset());
                    self.dmstatus.set_allhavereset(hart0.havereset());

                    self.dmstatus.set_anyresumeack(hart0.resume_ack());
                    self.dmstatus.set_allresumeack(hart0.resume_ack());

                    self.dmstatus.set_anyhalted(hart0.halted());
                    self.dmstatus.set_allhalted(hart0.halted());

                    self.dmstatus.set_anyrunning(hart0.running());
                    self.dmstatus.set_allrunning(hart0.running());

                    u32::from(self.dmstatus) as u64
                }
                HARTINFO_ADDR => u32::from(self.hartinfo) as u64,
                ABSTRACTCS_ADDR => u32::from(self.abstractcs) as u64,
                COMMAND_ADDR => 0,
                HALTSUM1_ADDR => 0,
                HAWINDOWSEL_ADDR => 0,
                HAWINDOW => 0,
                ABSTRACTAUTO_ADDR => 0,
                _ => {
                    debug!("unimplemented dmi_read: {:x}", addr);
                    0
                }
            }
        };

        trace!("dmi_read: {} {:x}", get_dm_register_name(addr), rdata);

        Some(rdata)
    }

    pub fn dmi_write(&mut self, address: u64, wdata: u64) -> Option<()> {
        let addr = (address - self.config.dm_base) as usize;

        trace!("dmi_write: {} {:x}", get_dm_register_name(addr), wdata);

        let abstract_data_range = core::ops::Range {
            start: ABSTRACT_DATA_BASE,
            end: ABSTRACT_DATA_BASE + self.config.abstract_data_count as usize,
        };
        let progbuf_range = core::ops::Range {
            start: PROGBUF_BASE,
            end: PROGBUF_BASE + self.config.progbuf_count as usize,
        };

        if abstract_data_range.contains(&addr) {
            let offset = addr - abstract_data_range.start;
            if let Some(value) = self.abstract_data.get_mut(offset) {
                *value = wdata as u32;
                Some(()) // success
            } else {
                debug!("abstract_data out of range: {:x}", offset);
                None // fail
            }
        } else if progbuf_range.contains(&addr) {
            let offset = addr - progbuf_range.start;
            if let Some(value) = self.progbuf.get_mut(offset) {
                *value = wdata as u32;
                Some(()) // success
            } else {
                debug!("program buffer out of range: {:x}", offset);
                None // fail
            }
        } else {
            match addr {
                DMCONTROL_ADDR => {
                    let new_dmcontrol = DMControl::from(wdata as u32);

                    if new_dmcontrol.dmactive() && !self.dmcontrol.dmactive() {
                        self.reset();
                    }
                    self.dmcontrol.set_dmactive(new_dmcontrol.dmactive());

                    if !self.dmcontrol.dmactive() || !self.dmstatus.authenticated() {
                        debug!(
                            "DM active {} , authenticated {}",
                            self.dmcontrol.dmactive(),
                            self.dmstatus.authenticated()
                        );
                    }
                    let mut hart0 = self.hart0.borrow_mut(); // only support single hart now

                    if new_dmcontrol.ackhavereset() {
                        // Clears havereset for any selected harts.
                        hart0.clear_havereset();
                    }

                    // Writing 0 clears the halt request bit for all currently selected harts.
                    // This may cancel outstanding halt requests for those harts.
                    // Writing 1 sets the halt request bit for all currently
                    // selected harts. Running harts will halt whenever
                    // their halt request bit is set.
                    hart0.set_haltreq(new_dmcontrol.haltreq());

                    if new_dmcontrol.haltreq() {
                        // halt request
                        debug!("DM: hart haltreq");
                    } else if new_dmcontrol.resumereq() {
                        // Writing 1 causes the currently selected harts to
                        // resume once, if they are halted when the write
                        // occurs. It also clears the resume ack bit for those
                        // harts.
                        // resumereq is ignored if haltreq is set.
                        debug!("DM: hart resumereq");
                        hart0.resumereq();
                    }

                    // This optional field writes the reset bit for all the
                    // currently selected harts. To perform a reset the
                    // debugger writes 1, and then writes 0 to deassert
                    // the reset signal.
                    hart0.set_reset_req(new_dmcontrol.hartreset());

                    // This bit controls the reset signal from the DM
                    // to the rest of the hardware platform. The signal
                    // should reset every part of the hardware platform,
                    // including every hart, except for the DM and any
                    // logic required to access the DM
                    hart0.set_reset_req(new_dmcontrol.ndmreset());

                    // TODO: is ndmreset and hartreset has priority?

                    self.dmcontrol = new_dmcontrol;
                    Some(())
                }
                DMSTATUS_ADDR => {
                    // read only
                    Some(())
                }
                HARTINFO_ADDR => {
                    self.hartinfo = HartInfo::from(wdata as u32);
                    Some(())
                }
                HALTSUM1_ADDR => Some(()),
                HAWINDOWSEL_ADDR => Some(()),
                HAWINDOW => Some(()),
                ABSTRACTCS_ADDR => {
                    let new_abstractcs = Abstractcs::from(wdata as u32);

                    if (new_abstractcs.busy()) {
                        // Writing this register while an abstract command is executing causes cmderr to become 1 (busy) once
                        self.abstractcs.set_cmderr(debug_const::CMDERR_BUSY as u8);
                    } else {
                        // this field remain set until they are cleared by writ-
                        // ing 1 to them.
                        let old_cmderr = self.abstractcs.cmderr();
                        let new_cmderr = old_cmderr & !new_abstractcs.cmderr();

                        // if new_abstractcs.cmderr() == 7 {
                        //     debug!("clear cmderr");
                        //     self.abstractcs.set_cmderr(0);
                        // }
                        self.abstractcs.set_cmderr(new_cmderr);
                    }

                    Some(())
                }
                COMMAND_ADDR => {
                    self.command = Command::from(wdata as u32);

                    // If cmderr is non-zero, writes to this register are ignored.
                    if self.abstractcs.cmderr() != debug_const::CMDERR_NONE as u8 {
                        debug!(
                            "Do not perform command when cmderr is not NONE, cmderr: {}",
                            self.abstractcs.cmderr()
                        );
                    } else {

                        // This bit is set as soon as command is written, and is not
                        // cleared until that command has completed.
                        self.abstractcs.set_busy(true);
                        self.perform_abstract_command();
                        self.abstractcs.set_busy(false);
                    }

                    Some(())
                }
                ABSTRACTAUTO_ADDR => Some(()),
                _ => {
                    debug!("unimplemented dmi_write: {:x}", addr);
                    None
                }
            }
        }
    }

    fn reset(&mut self) {
        // self.dmcontrol = DMControl::new();
        // self.dmstatus = DMStatus::new()
        //     .with_version(debug_module_register::debug_const::DMSTATUS_VERSION0_13 as u8)
        //     .with_authenticated(true);
        // self.hartinfo = HartInfo::new();
        // self.abstractcs = Abstractcs::new()
        //     .with_progbufsize(0)
        //     .with_datacount(self.config.abstract_data_count as u8);
        // self.command = Command::new();

        debug!("DM: reset unimplemented");
    }

    fn arg_read32(&self, arg_idx: u8) -> u32 {
        assert!(arg_idx < 3);
        self.abstract_data[arg_idx as usize]
    }
    fn arg_write32(&mut self, arg_idx: u8, value: u32) {
        assert!(arg_idx < 3);
        self.abstract_data[arg_idx as usize] = value;
    }
    fn arg_read64(&self, arg_idx: u8) -> u64 {
        assert!(arg_idx < 3);
        let idx = (arg_idx * 2) as usize; // 0,1; 2,3; 4,5
        u64::from(self.abstract_data[idx]) | (u64::from(self.abstract_data[idx + 1]) << 32)
    }
    fn arg_write64(&mut self, arg_idx: u8, value: u64) {
        assert!(arg_idx < 3);
        let idx = (arg_idx * 2) as usize; // 0,1; 2,3; 4,5
        self.abstract_data[idx] = value as u32;
        self.abstract_data[idx + 1] = (value >> 32) as u32;
    }

    fn perform_abstract_command(&mut self) {
        // only support single hart now
        let binding = self.hart0.clone();
        let mut hart0 = binding.borrow_mut();

        // The abstract command couldn’t
        // execute because the hart wasn’t in the required
        // state (running/halted), or unavailable.
        if !hart0.halted() {
            debug!("Do not perform command when hart is not halted",);

            self.abstractcs
                .set_cmderr(debug_const::CMDERR_HALT_RESUME as u8);
            return;
        }

        match self.command.cmdtype() as usize {
            debug_const::COMDTYPE_ACCESS_REG => {
                let command_reg = self.command.cmd_reg();

                if command_reg.transfer() {
                    let reg_data = match command_reg.regno() {
                        0x0000..=0x0fff => {
                            let csr_address = (command_reg.regno()) as usize;
                            if command_reg.write() {
                                match command_reg.aarsize() as usize {
                                    debug_const::AARSIZE_32 => {
                                        let wdata = self.arg_read32(0) as u64;
                                        hart0.write_csr(csr_address, wdata);
                                        Some(wdata)
                                    }
                                    debug_const::AARSIZE_64 => {
                                        let wdata = self.arg_read64(0);
                                        hart0.write_csr(csr_address, wdata);
                                        Some(wdata)
                                    }
                                    _ => {
                                        debug!("unimplemented aarsize: {}", command_reg.aarsize());
                                        None
                                    }
                                }
                            } else {
                                Some(hart0.read_csr(command_reg.regno().into()))
                            }
                        }
                        0x1000..=0x101f => {
                            let address = (command_reg.regno() - 0x1000) as usize;

                            if command_reg.write() {
                                match command_reg.aarsize() as usize {
                                    debug_const::AARSIZE_32 => {
                                        let wdata = self.arg_read32(0) as u64;
                                        hart0.write_gpr(address, wdata);
                                        Some(wdata)
                                    }
                                    debug_const::AARSIZE_64 => {
                                        let wdata = self.arg_read64(0);
                                        hart0.write_gpr(address, wdata);
                                        Some(wdata)
                                    }
                                    _ => {
                                        debug!("unimplemented aarsize: {}", command_reg.aarsize());
                                        None
                                    }
                                }
                            } else {
                                // read
                                Some(hart0.read_gpr(address))
                            }
                        }
                        0x1020..=0x103f => {
                            debug!("unimplemented register access: {:x}", command_reg.regno());
                            None
                        }
                        _ => {
                            debug!("unimplemented register access: {:x}", command_reg.regno());
                            // self.abstractcs.set_cmderr(0);
                            None
                        }
                    };

                    if let Some(reg_data_in) = reg_data {
                        if !command_reg.write() {
                            // self.abstract_data[0] = reg_data_in as u32;
                            // self.abstract_data[1] = (reg_data_in >> 32) as u32;

                            self.arg_write64(0, reg_data_in);
                        }
                    } else {
                        self.abstractcs.set_cmderr(debug_const::CMDERR_NOTSUP as u8);
                    }
                }
            }
            debug_const::COMDTYPE_QUICK_ACCESS => {
                debug!("perform_abstract_command: COMDTYPE_QUICK_ACCESS");
                self.abstractcs.set_cmderr(debug_const::CMDERR_NOTSUP as u8);
            }
            debug_const::COMDTYPE_ACCESS_MEM => {
                let command_mem = self.command.cmd_mem();

                if command_mem.aamvirtual() {
                    self.abstractcs.set_cmderr(debug_const::CMDERR_NOTSUP as u8);
                } else {
                    // physical memory access
                    match command_mem.aamsize() as usize {
                        debug_const::AAMSIZE_8 => {
                            let address = self.arg_read64(1);
                            if command_mem.write() {
                                let wdata = self.arg_read64(0);
                                if hart0.write_memory(address, 1, wdata).is_none() {
                                    debug!("write_memory failed, address: {:x}", address);
                                    self.abstractcs.set_cmderr(debug_const::CMDERR_BUS as u8);
                                }
                            } else {
                                match hart0.read_memory(address, 1) {
                                    Some(rdata) => self.arg_write64(0, rdata),
                                    None => {
                                        debug!("read_memory failed, address: {:x}", address);
                                        self.abstractcs.set_cmderr(debug_const::CMDERR_BUS as u8)
                                    }
                                };
                            }
                        }
                        debug_const::AAMSIZE_16 => {
                            let address = self.arg_read64(1);
                            if command_mem.write() {
                                let wdata = self.arg_read64(0);
                                if hart0.write_memory(address, 2, wdata).is_none() {
                                    debug!("write_memory failed, address: {:x}", address);
                                    self.abstractcs.set_cmderr(debug_const::CMDERR_BUS as u8);
                                }
                            } else {
                                match hart0.read_memory(address, 2) {
                                    Some(rdata) => self.arg_write64(0, rdata),
                                    None => {
                                        debug!("read_memory failed, address: {:x}", address);
                                        self.abstractcs.set_cmderr(debug_const::CMDERR_BUS as u8)
                                    }
                                };
                            }
                        }
                        debug_const::AAMSIZE_32 => {
                            let address = self.arg_read64(1);
                            if command_mem.write() {
                                let wdata = self.arg_read64(0);
                                if hart0.write_memory(address, 4, wdata).is_none() {
                                    debug!("write_memory failed, address: {:x}", address);
                                    self.abstractcs.set_cmderr(debug_const::CMDERR_BUS as u8);
                                }
                            } else {
                                match hart0.read_memory(address, 4) {
                                    Some(rdata) => self.arg_write64(0, rdata),
                                    None => {
                                        debug!("read_memory failed, address: {:x}", address);
                                        self.abstractcs.set_cmderr(debug_const::CMDERR_BUS as u8)
                                    }
                                };
                            }
                        }
                        debug_const::AAMSIZE_64 => {
                            let address = self.arg_read64(1);
                            if command_mem.write() {
                                let wdata = self.arg_read64(0);
                                if hart0.write_memory(address, 8, wdata).is_none() {
                                    debug!("write_memory failed, address: {:x}", address);
                                    self.abstractcs.set_cmderr(debug_const::CMDERR_BUS as u8);
                                }
                            } else {
                                match hart0.read_memory(address, 8) {
                                    Some(rdata) => self.arg_write64(0, rdata),
                                    None => {
                                        debug!("read_memory failed, address: {:x}", address);
                                        self.abstractcs.set_cmderr(debug_const::CMDERR_BUS as u8)
                                    }
                                };
                            }
                        }
                        _ => {
                            debug!("unimplemented aamsize: {}", command_mem.aamsize());
                            self.abstractcs.set_cmderr(debug_const::CMDERR_NOTSUP as u8);
                        }
                    }
                }
            }
            _ => {
                debug!("unimplemented command type: {}", self.command.cmdtype());
                self.abstractcs.set_cmderr(debug_const::CMDERR_NOTSUP as u8);
            }
        };
    }
}
