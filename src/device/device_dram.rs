use std::{fs, vec};

use log::info;

use crate::device::device_trait::DeviceBase;

pub struct DeviceDram {
    data: Vec<u8>,
    pub capacity: usize,
}

impl DeviceDram {
    pub fn new(size: usize) -> Self {
        let datavec: Vec<u8> = vec![0; size];

        DeviceDram {
            data: datavec,
            capacity: size,
        }
    }

    pub fn load_binary(&mut self, file_name: &str) {

        let read_ret = fs::read(file_name);

        let text = match read_ret {
            Ok(buff) => buff,
            Err(e) => panic!("can not read file:{e},{file_name}"),
        };

        let text_size = text.len();
        info!("load binary : {file_name}, size: {text_size}");
        self.data[0..text_size].copy_from_slice(&text[..text_size]);
    }
}

impl DeviceBase for DeviceDram {
    fn do_read(&mut self, addr: u64, len: usize) -> u64 {
        let mut data_bytes = 0_u64.to_le_bytes();

        data_bytes[..(len)].copy_from_slice(&self.data[(addr as usize)..(addr as usize + len)]);

        u64::from_le_bytes(data_bytes)
    }

    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        let data_bytes = data.to_le_bytes();

        self.data[(addr as usize)..(addr as usize + len)].copy_from_slice(&data_bytes[..(len)]);
        data
    }

    fn get_name(&self) -> &'static str {
        "DRAM"
    }
}

#[cfg(test)]
mod tests_dram {
    use super::*;

    #[test]
    fn test_do_read() {
        let mut dram = DeviceDram::new(1024);
        let data = 0xDEADBEEF;
        let data1 = 0xDEADBEEFDEADBEEF_u128;
        let len = 4;

        // write data to dram
        let addr = 0x100;
        dram.do_write(addr, data, len);
        dram.do_write(addr + 4, data, len);

        // read data from dram
        let result = dram.do_read(addr, len);
        // check if the read data is equal to the written data
        assert_eq!(result, data);

        let result = dram.do_read(addr, 1);
        assert_eq!(result, data & 0xff);
        let result = dram.do_read(addr, 2);
        assert_eq!(result, data & 0xffff);
        let result = dram.do_read(addr, 4);
        assert_eq!(result, data);
        let result = dram.do_read(addr, 8);
        // warn!("{:x}\n{:x}", result, data1);
        assert_eq!(result as u128, data1);
    }
}
