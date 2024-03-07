use bitfield_struct::bitfield;

pub const ABSTRACT_DATA_BASE: usize = 0x04;
pub const DMCONTROL_ADDR: usize = 0x10;
pub const DMSTATUS_ADDR: usize = 0x11;
pub const HARTINFO_ADDR: usize = 0x12;
pub const HALTSUM1_ADDR: usize = 0x13;
pub const HAWINDOWSEL_ADDR: usize = 0x14;
pub const HAWINDOW: usize = 0x15;
pub const ABSTRACTCS_ADDR: usize = 0x16;
pub const COMMAND_ADDR: usize = 0x17;
pub const ABSTRACTAUTO_ADDR: usize = 0x18;
pub const PROGBUF_BASE: usize = 0x20;

pub mod debug_const {
    pub const DMSTATUS_VERSION_NONE: usize = 0;
    pub const DMSTATUS_VERSION0_11: usize = 1;
    pub const DMSTATUS_VERSION0_13: usize = 2;
    pub const DMSTATUS_VERSION1_0: usize = 3;

    // Command types
    pub const COMDTYPE_ACCESS_REG: usize = 0;
    pub const COMDTYPE_QUICK_ACCESS: usize = 1;
    pub const COMDTYPE_ACCESS_MEM: usize = 2;

    // aarsize
    pub const AARSIZE_32: usize = 2;
    pub const AARSIZE_64: usize = 3;
    pub const AARSIZE_128: usize = 4;

    // aamsize
    pub const AAMSIZE_8: usize = 0;
    pub const AAMSIZE_16: usize = 1;
    pub const AAMSIZE_32: usize = 2;
    pub const AAMSIZE_64: usize = 3;
    pub const AAMSIZE_128: usize = 4;

    // cmderr
    pub const CMDERR_NONE: usize = 0;
    pub const CMDERR_BUSY: usize = 1;
    pub const CMDERR_NOTSUP: usize = 2;
    pub const CMDERR_EXCEPTION: usize = 3;
    pub const CMDERR_HALT_RESUME: usize = 4;
    pub const CMDERR_BUS: usize = 5;
    pub const CMDERR_RESERVED: usize = 6;
    pub const CMDERR_OTHER: usize = 7;

    // sbaccess
    pub const SBACCESS_8: usize = 0;
    pub const SBACCESS_16: usize = 1;
    pub const SBACCESS_32: usize = 2;
    pub const SBACCESS_64: usize = 3;
    pub const SBACCESS_128: usize = 4;

    // sberror
    pub const SBERROR_NONE: usize = 0;
    pub const SBERROR_TIMEOUT: usize = 1;
    pub const SBERROR_BADADDR: usize = 2;
    pub const SBERROR_ALIGNMENT: usize = 3;
    pub const SBERROR_SIZE: usize = 4;
    pub const SBERROR_OTHER: usize = 7;
}

pub fn get_dm_register_name(addr: usize) -> &'static str {
    match addr {
        ABSTRACT_DATA_BASE => "ABSTRACT_DATA",
        DMCONTROL_ADDR => "DMCONTROL",
        DMSTATUS_ADDR => "DMSTATUS",
        HARTINFO_ADDR => "HARTINFO",
        HALTSUM1_ADDR => "HALTSUM1",
        HAWINDOWSEL_ADDR => "HAWINDOWSEL",
        HAWINDOW => "HAWINDOW",
        ABSTRACTCS_ADDR => "ABSTRACTCS",
        COMMAND_ADDR => "COMMAND",
        ABSTRACTAUTO_ADDR => "ABSTRACTAUTO",
        PROGBUF_BASE => "PROGBUF",
        _ => "UNKNOWN",
    }
}

#[bitfield(u32)]
pub struct DMStatus {
    #[bits(4)]
    pub version: u8,
    pub confstrptrvalid: bool,
    pub hasresethaltreq: bool,
    pub authbusy: bool,
    pub authenticated: bool,
    pub anyhalted: bool,
    pub allhalted: bool,
    pub anyrunning: bool,
    pub allrunning: bool,
    pub anyunavail: bool,
    pub allunavail: bool,
    pub anynonexistent: bool,
    pub allnonexistent: bool,
    pub anyresumeack: bool,
    pub allresumeack: bool,
    pub anyhavereset: bool,
    pub allhavereset: bool,
    #[bits(2)]
    pub zero0: u8,
    pub impebreak: bool,
    pub stickyunavail: bool,
    pub ndmresetpending: bool,
    #[bits(7)]
    pub zero1: u8,
}

#[bitfield(u32)]
pub struct DMControl {
    pub dmactive: bool,
    pub ndmreset: bool,
    pub clrresethaltreq: bool,
    pub setresethaltreq: bool,
    pub clrkeepalive: bool,
    pub setkeepalive: bool,
    #[bits(10)]
    pub hartselhi: u16,
    #[bits(10)]
    pub hartsello: u16,
    pub hasel: bool,
    pub ackunavail: bool,
    pub ackhavereset: bool,
    pub hartreset: bool,
    pub resumereq: bool,
    pub haltreq: bool,
}

#[bitfield(u32)]
pub struct HartInfo {
    #[bits(12)]
    pub dataaddr: u16,
    #[bits(4)]
    pub datasize: u8,
    pub dataaccess: bool,
    #[bits(3)]
    pub zero0: u8,
    #[bits(4)]
    pub nscratch: u8,
    pub zero1: u8,
}

#[bitfield(u32)]
pub struct HaWindowSel {
    #[bits(15)]
    hawindowsel: u16,
    #[bits(17)]
    zero0: u32,
}

#[bitfield(u32)]
pub struct Abstractcs {
    #[bits(4)]
    pub datacount: u8,
    #[bits(4)]
    pub zero0: u8,
    #[bits(3)]
    pub cmderr: u8,
    pub relaxedpriv: bool,
    pub busy: bool,
    #[bits(11)]
    pub zero1: u16,
    #[bits(5)]
    pub progbufsize: u8,
    #[bits(3)]
    pub zero2: u8,
}

#[bitfield(u32)]
pub struct Command {
    #[bits(24)]
    pub control: u32,
    pub cmdtype: u8,
}

impl Command {
    pub fn cmd_reg(&self) -> CommandReg {
        CommandReg::from(self.0)
    }
    pub fn cmd_mem(&self) -> CommandMem {
        CommandMem::from(self.0)
    }
}

#[bitfield(u32)]
pub struct CommandReg {
    pub regno: u16,
    pub write: bool,
    pub transfer: bool,
    pub postexec: bool,
    pub aarpostincrement: bool,
    #[bits(3)]
    pub aarsize: u8,
    pub zero0: bool,
    pub cmdtype: u8,
}

#[bitfield(u32)]
pub struct CommandMem {
    #[bits(14)]
    pub zero0: u16,
    #[bits(2)]
    pub target_specific: u8,
    pub write: bool,
    #[bits(2)]
    pub zero1: u8,
    pub aampostincrement: bool,
    #[bits(3)]
    pub aamsize: u8,
    pub aamvirtual: bool,
    pub cmdtype: u8,
}

#[bitfield(u32)]
pub struct AbstractAuto {
    #[bits(12)]
    autoexecdata: u16,
    #[bits(4)]
    zero0: u8,
    autoexecprogbuf: u16,
}

#[bitfield(u32)]
pub struct DMCs2 {
    hgselect: bool,
    hgwrite: bool,
    #[bits(5)]
    group: u8,
    #[bits(4)]
    dmexttrigger: u8,
    grouptype: bool,
    #[bits(20)]
    zero0: u32,
}

// System Bus Access Control and Status Register
#[bitfield(u32)]
pub struct SBCS {
    pub sbaccess8: bool,
    pub sbaccess16: bool,
    pub sbaccess32: bool,
    pub sbaccess64: bool,
    pub sbaccess128: bool,
    #[bits(7)]
    pub sbasize: u8,
    #[bits(3)]
    pub sberror: u8,
    pub sbreadondata: bool,
    pub sbautoincrement: bool,
    #[bits(3)]
    pub sbaccess: u8,
    pub sbreadonaddr: bool,
    pub sbbusy: bool,
    pub sbbusyerror: bool,
    #[bits(6)]
    pub zero0: u8,
    #[bits(3)]
    pub sbversion: u8,
}
