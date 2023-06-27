
use crate::{device::device_trait::DeviceBase, tools::FifoUnbounded};

pub struct DeviceUart {
    txfifo: FifoUnbounded<u8>,
}

impl DeviceUart {
    pub fn new(uart_tx: FifoUnbounded<u8>) -> Self {
        DeviceUart { txfifo: uart_tx }
    }
}

impl DeviceBase for DeviceUart {
    fn do_read(&mut self, _addr: u64, _len: usize) -> u64 {
        panic!("This uart is write only");
    }

    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        assert_eq!(len, 1);
        assert_eq!(addr, 0);
        self.txfifo.push(data as u8);
        0
    }

    fn get_name(&self) -> &'static str {
        "AM_UART"
    }
}
