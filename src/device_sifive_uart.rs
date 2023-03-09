use std::io::{self, Write};


use ring_channel::RingReceiver;

use crate::device_trait::DeviceBase;

const TXDATA: usize = 0x00;
const RXDATA: usize = 0x04;
const TXCTRL: usize = 0x08;
const RXCTRL: usize = 0x0c;
const IE: usize = 0x10;
const IP: usize = 0x14;
const DIV: usize = 0x18;

// const TXDATA_OFF: usize = TXDATA / 4;
// const RXDATA_OFF: usize = RXDATA / 4;
// const TXCTRL_OFF: usize = TXCTRL / 4;
// const RXCTRL_OFF: usize = RXCTRL / 4;
// const IE_OFF: usize = IE / 4;
// const IP_OFF: usize = IP / 4;
// const DIV_OFF: usize = DIV / 4;

pub struct DeviceSifiveUart {
    txdata: u32,
    rxdata: u32,
    txctrl: u32,
    rxctrl: u32,
    ie: u32,
    ip: u32,
    div: u32,
    rxbuf: RingReceiver<i32>,
}

impl DeviceSifiveUart {
    pub fn new(rxbuf: RingReceiver<i32>) -> Self {
        DeviceSifiveUart {
            txdata: 0,
            rxdata: 0,
            txctrl: 0,
            rxctrl: 0,
            ie: 0,
            ip: 0,
            div: 0,
            rxbuf,
        }
    }

    pub fn put_char(&mut self, ch: u64) {
        let c = char::from_u32(ch as u32).unwrap();
        print!("{c}");
        io::stdout().flush().unwrap();
    }
}

impl DeviceBase for DeviceSifiveUart {
    fn do_read(&mut self, addr: u64, len: usize) -> u64 {
        assert_eq!(len, 4);

        match addr as usize {
            RXDATA => self.rxbuf.try_recv().map_or(0, |x| x as u64),
            _ => 0,
        }
    }

    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        assert_eq!(len, 4);

        match addr as usize {
            TXDATA => {
                self.put_char(data);
                0
            }
            _ => 0,
        }
    }

    fn get_name(&self) -> &'static str {
        "SIFIVE_UART"
    }

    fn do_update(&mut self) {}
}

// #[test]

// fn term_test1() {
//     let term = Term::stdout();

//     loop {
//         if let Ok(val) = term.read_char() {
//             print!("{val}");
//             io::stdout().flush();
//         }
//     }
// }
