use bitfield_struct::bitfield;

use crate::device::device_trait::DeviceBase;

const RBR: u64 = 0x00; // Receive Buffer Register (read only)
const THR: u64 = 0x00; // Transmit Holding Register (write only)
const IER: u64 = 0x01; // Interrupt Enable Register (read/write)
const IIR: u64 = 0x02; // Interrupt Identification Register (read only)
const FCR: u64 = 0x02; // FIFO Control Register (write only)
const LCR: u64 = 0x03; // Line Control Register (read/write)
const MCR: u64 = 0x04; // Modem Control Register (write)
const LSR: u64 = 0x05; // Line Status Register (read/write)
const MSR: u64 = 0x06; // Modem Status Register (read/write)
const SCR: u64 = 0x07; // Scratch Register (read/write)

#[bitfield(u8)]
struct Ier {
    pub rx_avali: bool,  // Received Data Available Interrupt Enable
    pub thr_empty: bool, // Transmitter Holding Register Empty Interrupt Enable
    pub rls: bool,       // Receiver Line Status Interrupt
    pub ms: bool,        // Modem Status Interrupt
    #[bits(4)]
    _reserved: u8,
}

// Bits 4 and 5: Logic ‘0’.
// Bits 6 and 7: Logic ‘1’ for compatibility reason.
// Reset Value: C1h
#[bitfield(u8)]
struct Iir {
    pub ip: bool, // Interrupt Pending
    #[bits(3)]
    ip_type: u8,
    #[bits(4)]
    _reserved: u8,
}
impl Default for Iir {
    fn default() -> Self {
        Iir::from(0xC1)
    }
}

// Reset Value : 11000000b
#[bitfield(u8)]
struct Fcr {
    pub enable: bool, // FIFO Enable
    pub rx_reset: bool,
    pub tx_reset: bool,
    #[bits(3)]
    _reserved: u8,
    #[bits(2)]
    rx_level: u8,
}
impl Default for Fcr {
    fn default() -> Self {
        Fcr::from(0xC0)
    }
}

//Reset Value: 00000011b
#[bitfield(u8)]
struct Lcr {
    #[bits(2)]
    pub character_length: u8,
    pub stop_bit: bool,
    pub parity_enable: bool,
    pub even_parity: bool,
    pub stick_parity: bool,
    pub set_break: bool,
    pub dlab: bool,
}
impl Default for Lcr {
    fn default() -> Self {
        Lcr::from(0x03)
    }
}

#[bitfield(u8)]
struct Mcr {
    pub dtr: bool,
    pub rts: bool,
    pub out1: bool,
    pub out2: bool,
    pub loopback: bool,
    #[bits(3)]
    _reserved: u8,
}
#[bitfield(u8)]
struct Lsr {
    pub data_ready: bool, // Rx Data Ready
    pub overrun_error: bool,
    pub parity_error: bool,
    pub framing_error: bool,
    pub break_interrupt: bool,
    pub thr_empty: bool,
    pub tsr_empty: bool,
    pub fifo_error: bool,
}
#[bitfield(u8)]
struct Msr {
    pub delta_cts: bool,
    pub delta_dsr: bool,
    pub teri: bool,
    pub delta_dcd: bool,
    pub cts: bool,
    pub dsr: bool,
    pub ri: bool,
    pub dcd: bool,
}

#[bitfield(u32)]
struct Txdata {
    pub data: u8,
    #[bits(23)]
    _reserved: u32,
    pub full: bool,
}

#[bitfield(u32)]
struct Rxdata {
    pub data: u8,
    #[bits(23)]
    _reserved: u32,
    pub empty: bool,
}

#[bitfield(u32)]
struct Txctrl {
    pub txen: bool,
    pub nstop: bool,
    #[bits(14)]
    _reserved0: u32,
    #[bits(3)]
    pub txcnt: u8,
    #[bits(13)]
    _reserved1: u32,
}

#[bitfield(u32)]
struct Rxctrl {
    pub rxen: bool,
    #[bits(15)]
    _reserved0: u32,
    #[bits(3)]
    pub rxcnt: u8,
    #[bits(13)]
    _reserved1: u32,
}

#[bitfield(u32)]
struct IeIp {
    pub txwm: bool,
    pub rxwm: bool,
    #[bits(30)]
    _reserved: u32,
}

struct Uart16550aIN {
    rbr: u8,
    thr: u8,
    ier: Ier,
    iir: Iir,
    fcr: Fcr,
    lcr: Lcr,
    mcr: Mcr,
    lsr: Lsr,
    msr: Msr,
    scr: u8,
}
impl Uart16550aIN {
    pub fn new() -> Self {
        Uart16550aIN {
            rbr: 0,
            thr: 0,
            ier: Ier::new(),
            iir: Iir::default(),
            fcr: Fcr::default(),
            lcr: Lcr::default(),
            mcr: Mcr::new(),
            lsr: Lsr::new(),
            msr: Msr::new(),
            scr: 0,
        }
    }
}

type Rxfifo = crossbeam_channel::Receiver<u8>;
type Txfifo = crossbeam_channel::Sender<u8>;
pub struct Device16550aUART {
    regs: Uart16550aIN,
    rxfifo: Rxfifo,
    txfifo: Txfifo,
}

impl Device16550aUART {
    pub fn new(uart_tx: Txfifo, uart_rx: Rxfifo) -> Self {
        Device16550aUART {
            regs: Uart16550aIN::new(),
            txfifo: uart_tx,
            rxfifo: uart_rx,
        }
    }

    fn read_lsr(&mut self) -> u8 {
        let mut lsr = self.regs.lsr;
        if self.rxfifo.is_empty() {
            lsr.set_data_ready(false);
        } else {
            lsr.set_data_ready(true);
        }
        lsr.0
    }

    pub fn put_char(&mut self, ch: u64) {
        // let c = char::from_u32(ch as u32).unwrap();
        // print!("{c}");
        // io::stdout().flush().unwrap();
        // println!("{:x}", ch);
        let c = ch as u8;
        self.txfifo.send(c).unwrap();
        self.regs.lsr.set_thr_empty(true);
    }

    fn get_char(&mut self) -> u8 {
        self.rxfifo.try_recv().unwrap_or(0)
    }
}

impl DeviceBase for Device16550aUART {
    fn do_read(&mut self, addr: u64, len: usize) -> u64 {
        assert_eq!(len, 1);
        match addr {
            RBR => self.get_char() as u64,
            IER => self.regs.ier.0 as u64,
            IIR => self.regs.iir.0 as u64,
            LCR => self.regs.lcr.0 as u64,
            LSR => self.read_lsr() as u64,
            MSR => self.regs.msr.0 as u64,
            SCR => self.regs.scr as u64,
            _ => panic!("invalid read address:{:x}", addr),
        }
    }

    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        assert_eq!(len, 1);
        match addr {
            THR => {
                self.regs.thr = data as u8;
                self.put_char(data);
            }
            IER => self.regs.ier = Ier::from(data as u8),
            FCR => self.regs.fcr = Fcr::from(data as u8),
            LCR => self.regs.lcr = Lcr::from(data as u8),
            MCR => self.regs.mcr = Mcr::from(data as u8),
            LSR => self.regs.lsr = Lsr::from(data as u8),
            MSR => self.regs.msr = Msr::from(data as u8),
            SCR => self.regs.scr = data as u8,
            _ => panic!("invalid write address:{:x}", addr),
        }
        0
    }

    fn get_name(&self) -> &'static str {
        "16550a UART"
    }
}
