use std::io::{self, Write};

use crate::device_trait::DeviceBase;

pub struct DeviceVGACTL {}

impl DeviceVGACTL {
    pub fn new() -> Self {
        DeviceVGACTL {}
    }
}

impl DeviceBase for DeviceVGACTL {
    fn do_read(&mut self, _addr: u64, _len: usize) -> u64 {
        0
    }

    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        0
    }

    fn get_name(& self) -> &'static str {
        "VGA_CTL"
    }
}
