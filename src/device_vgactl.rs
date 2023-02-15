use ring_channel::RingSender;

use crate::device_trait::DeviceBase;

pub struct DeviceVGACTL {
    tx: RingSender<bool>,
}

impl DeviceVGACTL {
    pub fn new(tx: RingSender<bool>) -> Self {
        DeviceVGACTL { tx }
    }
}

impl DeviceBase for DeviceVGACTL {
    fn do_read(&mut self, _addr: u64, _len: usize) -> u64 {
        panic!("should not read !!");
    }

    fn do_write(&mut self, addr: u64, _data: u64, len: usize) -> u64 {
        assert_eq!(addr, 4);
        assert_eq!(len, 4);
        self.tx.send(true).unwrap();
        0
    }

    fn get_name(&self) -> &'static str {
        "VGA_CTL"
    }
}
