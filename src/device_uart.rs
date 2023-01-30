use std::io::{self, Write};

use crate::device_trait::DeviceBase;

pub struct DeviceUart {}

impl DeviceBase for DeviceUart {
    fn do_read(&mut self, addr: u64, len: usize) -> u64 {
        panic!();
    }

    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        assert_eq!(len, 1);
        assert_eq!(addr, 0);

        let c = char::from_u32(data as u32).unwrap();

        print!("{c}");
        io::stdout().flush().unwrap();
        c as u64
    }
}

#[cfg(test)]
mod test_gpr {
    use crate::device_trait::DeviceBase;

    use super::DeviceUart;


    #[test]
    fn tset1() {

        let mut uart = DeviceUart{};


        let hello = "hello\n";

        for s in hello.chars() {
            // println!("byte:{}",s as u32);
            uart.do_write(0, s as u64, 1);
        }
        
    }
}
