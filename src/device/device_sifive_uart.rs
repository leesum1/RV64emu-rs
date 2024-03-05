use core::cell::Cell;

use alloc::{boxed::Box, rc::Rc};
use bitfield_struct::bitfield;

use crate::{device::device_trait::DeviceBase, tools::FifoUnbounded};

const TXDATA: usize = 0x00;
const RXDATA: usize = 0x04;
const TXCTRL: usize = 0x08;
const RXCTRL: usize = 0x0c;
const IE: usize = 0x10;
const IP: usize = 0x14;
const DIV: usize = 0x18;

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

struct SifiveUartIN {
    txdata: Txdata,
    rxdata: Rxdata,
    txctrl: Txctrl,
    rxctrl: Rxctrl,
    ie: IeIp,
    ip: IeIp,
    div: u32,
}

impl SifiveUartIN {
    pub fn new() -> Self {
        SifiveUartIN {
            txdata: 0.into(),
            rxdata: 0.into(),
            txctrl: 0.into(),
            rxctrl: 0.into(),
            ie: 0.into(),
            ip: 0.into(),
            div: 0,
        }
    }
}

pub struct DeviceSifiveUart {
    regs: Box<SifiveUartIN>,
    pub irq_pending: Rc<Cell<bool>>,

    rxfifo: FifoUnbounded<u8>,
    txfifo: FifoUnbounded<u8>,
}

impl DeviceSifiveUart {
    pub fn new(uart_tx: FifoUnbounded<u8>, uart_rx: FifoUnbounded<u8>) -> Self {
        DeviceSifiveUart {
            regs: Box::new(SifiveUartIN::new()),
            txfifo: uart_tx,
            rxfifo: uart_rx,
            irq_pending: Rc::new(Cell::new(false)),
        }
    }
    pub fn put_char(&mut self, ch: u64) {
        // let c = char::from_u32(ch as u32).unwrap();
        // print!("{c}");
        // io::stdout().flush().unwrap();
        let c = ch as u8;
        // self.txfifo.send(c).unwrap();
        self.txfifo.push(c);
    }

    pub fn get_rxdata(&mut self) -> u32 {
        let empty = self.rxfifo.is_empty();
        // let data = self.rxfifo.try_recv().map_or(0, |x| x as u64);
        let data = self.rxfifo.pop().map_or(0, |x| x as u64);

        self.regs.rxdata = Rxdata::new().with_empty(empty).with_data(data as u8);
        // debug!("sifive_uart: get_rxdata: {:?}", self.regs.rxdata);
        self.regs.rxdata.0
    }
    pub fn get_txdata(&mut self) -> u32 {
        // we don't have a real txdata register, so we just return 0
        self.regs.txdata.set_full(false);
        // debug!("sifive_uart: get_txdata: {:?}", self.regs.txdata);
        self.regs.txdata.0
    }
}

impl DeviceBase for DeviceSifiveUart {
    fn do_read(&mut self, addr: u64, len: usize) -> u64 {
        assert_eq!(len, 4);
        match addr as usize {
            TXDATA => self.get_txdata() as u64,
            RXDATA => self.get_rxdata() as u64,
            TXCTRL => self.regs.txctrl.0 as u64,
            RXCTRL => self.regs.rxctrl.0 as u64,
            IE => self.regs.ie.0 as u64,
            IP => self.regs.ip.0 as u64,
            DIV => self.regs.div as u64,
            _ => panic!("sifive_uart: unknown addr: {:#x}", addr),
        }
    }
    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        assert_eq!(len, 4);
        match addr as usize {
            TXDATA => self.put_char(data),
            TXCTRL => self.regs.txctrl.0 = data as u32,
            RXCTRL => self.regs.rxctrl.0 = data as u32,
            IE => {
                // debug!("sifive_uart: IE: {:#x}", data);
                self.regs.ie.0 = data as u32
            }
            RXDATA | IP => {} // read only
            DIV => self.regs.div = data as u32,
            _ => panic!("sifive_uart: unknown addr: {:#x}", addr),
        };

        0
    }

    fn get_name(&self) -> &'static str {
        "SIFIVE_UART"
    }

    fn do_update(&mut self) {
        let rxwm_pending = self.rxfifo.len() > self.regs.rxctrl.rxcnt().into();
        let txwm_pending = self.txfifo.len() < self.regs.txctrl.txcnt().into();

        self.regs.ip.set_rxwm(rxwm_pending);
        self.regs.ip.set_txwm(txwm_pending);

        // update irq_pending
        let has_irq = 0 != (self.regs.ip.0 & self.regs.ie.0);
        // println!("ie:{:?}", self.regs.ie);
        // debug!("sifive_uart: irq_pending: {}", has_irq);
        self.irq_pending.set(has_irq);
    }
}
