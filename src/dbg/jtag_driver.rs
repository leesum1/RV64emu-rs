use std::fmt::Debug;

use bitfield_struct::bitfield;
use log::{debug, trace};

use super::{debug_module::DebugModule, jtag_state::JtagState};


const DMI_OP_STATUS_SUCCESS: u8 = 0;
const DMI_OP_STATUS_RESERVED: u8 = 1;
const DMI_OP_STATUS_FAILED: u8 = 2;
const DMI_OP_STATUS_BUSY: u8 = 3;

const DMI_OP_NOP: u8 = 0;
const DMI_OP_READ: u8 = 1;
const DMI_OP_WRITE: u8 = 2;
const DMI_OP_RESERVED: u8 = 3;

const DMI_OP_SUCCESS: u8 = 0;
const DMI_OP_FAILED: u8 = 2;

#[bitfield(u32)]
struct DTMIDCode {
    _pad: bool,
    #[bits(11)]
    manufld: u16,
    #[bits(16)]
    part_number: u16,
    #[bits(4)]
    version: u8,
}

#[bitfield(u32)]
struct DTMCS {
    #[bits(4)]
    version: u8,
    #[bits(6)]
    abits: u8,
    #[bits(2)]
    dmistat: u8,
    #[bits(3)]
    idle: u8,
    _pad: bool,
    dmireset: bool,
    dtmhardreset: bool,
    #[bits(3)]
    errinfo: u8,
    #[bits(11)]
    _pad: u16,
}

#[bitfield(u64)]
struct DTMI {
    #[bits(2)]
    op: u8,
    data: u32,
    #[bits(30)]
    address: u32,
}

#[allow(clippy::upper_case_acronyms)]
pub enum JtagDTMRegister {
    BYPASS0 = 0x00,
    IDCODE = 0x01,
    DTMCS = 0x10,
    DMI = 0x11,
    BYPASS1F = 0x1f,
}

impl JtagDTMRegister {
    fn new(val: u8) -> JtagDTMRegister {
        match val {
            0x00 => JtagDTMRegister::BYPASS0,
            0x01 => JtagDTMRegister::IDCODE,
            0x10 => JtagDTMRegister::DTMCS,
            0x11 => JtagDTMRegister::DMI,
            0x1f => JtagDTMRegister::BYPASS1F,
            _ => panic!("Invalid JtagDTMRegister value: {}", val),
        }
    }
}

struct ShifterRegister {
    data: u64,
    len: u8,
    default: u64,
}

impl ShifterRegister {
    fn new(default: u64, len: u8) -> ShifterRegister {
        ShifterRegister {
            data: default,
            len,
            default,
        }
    }

    fn clear(&mut self) {
        self.data = 0;
    }
    fn len_mask(&self) -> u64 {
        (1 << self.len) - 1
    }

    fn set(&mut self, new_data: u64, len: u8) {
        self.len = len;
        self.data = new_data & self.len_mask()
    }

    fn shift_right(&mut self, new_in: bool) {
        self.data >>= 1;
        self.data |= (new_in as u64) << (self.len - 1);
    }
    fn shift_left(&mut self, new_in: bool) {
        self.data <<= 1;
        self.data |= new_in as u64;
    }

    fn data(&self) -> u64 {
        self.data & self.len_mask()
    }
}

pub struct JtagDriver {
    //  debug module
    dm: DebugModule,

    // jtag wires
    tck: bool,
    tms: bool,
    tdi: bool,
    tdo: bool,

    // jtag shifter regs
    ir: ShifterRegister,
    dr: ShifterRegister,

    // dtm regs
    dtmcs: DTMCS,
    dmi: DTMI,
    idcode: DTMIDCode,
    bypass0: bool,

    state: JtagState,
}

impl JtagDriver {
    pub fn new(dm: DebugModule) -> JtagDriver {
        let dtmcontrol = DTMCS::new()
            .with_errinfo(0)
            .with_version(1) // 0.13
            .with_abits(6) //The size of address in dmi.
            .with_dmistat(0);

        let dmi = DTMI::new().with_op(DMI_OP_SUCCESS);

        JtagDriver {
            dm,
            tck: false,
            tms: false,
            tdi: false,
            tdo: false,
            ir: ShifterRegister::new(0, 6),
            dr: ShifterRegister::new(0, 64),
            dtmcs: dtmcontrol,
            dmi,
            idcode: DTMIDCode::from(0xdeadbeef),
            bypass0: false,
            state: JtagState::TestLogicReset,
        }
    }

    pub fn get_tdo(&self) -> bool {
        self.tdo
    }

    pub fn set_pins(&mut self, new_tck: bool, new_tms: bool, new_tdi: bool) {
        let posedge_tck = new_tck && !self.tck;

        if posedge_tck {
            // Positive clock edge. TMS and TDI are sampled on the rising edge of TCK by
            // Target.
            match self.state {
                JtagState::ShiftDr => {
                    self.dr.shift_right(self.tdi);
                }
                JtagState::ShiftIr => {
                    self.ir.shift_right(self.tdi);
                }
                _ => {}
            }
            self.state = self.state.next_state(self.tms);
        } else {
            // Negative clock edge. TDO is updated.
            match self.state {
                JtagState::RunTestIdle => {}
                JtagState::TestLogicReset => {
                    // This register is selected (in IR) when the TAP state machine is reset
                    self.ir.set(JtagDTMRegister::IDCODE as u64, 32);
                }
                JtagState::CaptureDr => {
                    self.capture_dr();
                }
                JtagState::UpdateDr => {
                    self.update_dr();
                }
                JtagState::CaptureIr => {
                    self.ir.set(0x01, 5); //JTAG spec only says must end with 'b01.
                }
                JtagState::ShiftDr => {
                    self.tdo = self.dr.data() & 1 != 0;
                }
                JtagState::ShiftIr => {
                    self.tdo = self.ir.data() & 1 != 0;
                }
                _ => {}
            }
        }

        self.tck = new_tck;
        self.tms = new_tms;
        self.tdi = new_tdi;
    }

    fn is_busy(&self) -> bool {
        false
    }

    fn capture_dr(&mut self) {
        match JtagDTMRegister::new(self.ir.data() as u8) {
            JtagDTMRegister::IDCODE => {
                self.dr.set(self.idcode.0.into(), 32);
            }
            JtagDTMRegister::DTMCS => {
                self.dr.set(self.dtmcs.0.into(), 32);
            }
            JtagDTMRegister::DMI => {
                // if self.is_busy() response with 3 (DMI_OP_STATUS_BUSY)
                let _tmp = if self.is_busy() {
                    DMI_OP_STATUS_BUSY as u64
                } else {
                    self.dmi.into()
                };
                self.dr.set(_tmp, 40);
            }
            JtagDTMRegister::BYPASS0 => {
                self.dr.set(self.bypass0 as u64, 1);
            }
            _ => {
                panic!("Invalid JtagDTMRegister: {:?}", self.ir.data());
            }
        }
        log::trace!(
            "Capture DR; ir: {:x}, dr: {:x}",
            self.ir.data(),
            self.dr.data()
        );
    }

    pub fn reset(&mut self) {
        self.ir.clear();
        self.dr.clear();
        self.dmi = DTMI::new();
        self.state = JtagState::TestLogicReset;
    }

    fn update_dr(&mut self) {
        log::trace!(
            "Updata DR; ir: {:x}, dr: {:x}",
            self.ir.data(),
            self.dr.data()
        );

        match JtagDTMRegister::new(self.ir.data() as u8) {
            JtagDTMRegister::IDCODE => {}
            JtagDTMRegister::DTMCS => {
                let dtmcs_tmp = DTMCS::from(self.dr.data() as u32);
                if dtmcs_tmp.dmireset() {
                    todo!("dmireset")
                }
                if dtmcs_tmp.dtmhardreset() {
                    todo!("dtmhardreset");
                }
            }

            JtagDTMRegister::BYPASS0 => {
                self.bypass0 = self.dr.data() != 0;
            }
            JtagDTMRegister::DMI => {
                self.dmi.0 = self.dr.data();
                match self.dmi.op() {
                    DMI_OP_NOP => {
                        // 通过 nop 来检查上一个操作的结果
                        log::trace!("DMI_OP_NOP");
                    }
                    DMI_OP_READ => {
                        let rdata = self.dm.dmi_read(self.dmi.address().into());
                        match rdata {
                            Some(data) => {
                                self.dmi.set_data(data as u32);
                                self.dmi.set_op(DMI_OP_SUCCESS);
                                trace!(
                                    "dmi_read success : {:x} {:x}",
                                    self.dmi.address(),
                                    self.dmi.data()
                                );
                            }
                            None => {
                                self.dmi.set_op(DMI_OP_FAILED);
                                trace!("dmi_read failed: {:x}", self.dmi.address());
                            }
                        }
                    }
                    DMI_OP_WRITE => {
                        let w_ret = self
                            .dm
                            .dmi_write(self.dmi.address().into(), self.dmi.data().into());

                        if w_ret.is_some() {
                            self.dmi.set_op(DMI_OP_SUCCESS);
                            trace!(
                                "dmi_write success : {:x} {:x}",
                                self.dmi.address(),
                                self.dmi.data()
                            );
                        } else {
                            self.dmi.set_op(DMI_OP_FAILED);
                            trace!(
                                "dmi_write failed: {:x} {:x}",
                                self.dmi.address(),
                                self.dmi.data()
                            );
                        }
                    }
                    DMI_OP_RESERVED => {
                        todo!("DMI_OP_RESERVED");
                    }
                    _ => {
                        panic!("Invalid DMI op: {}", self.dmi.op());
                    }
                }
            }
            _ => {
                panic!("Invalid JtagDTMRegister: {:?}", self.ir.data());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shift_register() {
        let mut reg = ShifterRegister::new(0, 8);

        reg.shift_right(true);
        assert_eq!(reg.data(), 128);
        reg.shift_right(false);
        assert_eq!(reg.data(), 64);
        reg.shift_right(true);
        assert_eq!(reg.data(), 128 + 32);
        reg.shift_right(false);
        assert_eq!(reg.data(), 64 + 16);
        reg.shift_right(true);
        assert_eq!(reg.data(), 128 + 32 + 8);
        reg.shift_right(false);
        assert_eq!(reg.data(), 64 + 16 + 4);
        reg.shift_right(true);
        assert_eq!(reg.data(), 128 + 32 + 8 + 2);
        reg.shift_right(false);
        assert_eq!(reg.data(), 64 + 16 + 4 + 1);
        reg.shift_right(true);
        assert_eq!(reg.data(), 128 + 32 + 8 + 2);
    }
}
