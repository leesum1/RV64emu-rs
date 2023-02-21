

use crate::device_trait::DeviceBase;
use chrono::prelude::*;

pub struct DeviceRTC {
    pub rtc_time: u64,
}

impl DeviceRTC {
    pub fn new() -> Self {
        DeviceRTC { rtc_time: 0 }
    }
}

impl DeviceBase for DeviceRTC {
    fn do_read(&mut self, addr: u64, len: usize) -> u64 {
        assert_eq!(len, 4);
        match addr {
            0 => {
                self.rtc_time = Local::now().timestamp_micros() as u64;
                self.rtc_time as u32 as u64
            }
            4 => self.rtc_time >> 32,
            _ => panic!("RTC addr err:{addr}"),
        }
    }

    fn do_write(&mut self, _addr: u64, _data: u64, _len: usize) -> u64 {
        panic!("RTC can not write")
    }

    fn get_name(&self) -> &'static str {
        "RTC"
    }
}

#[cfg(test)]
mod test_rtc {

    use crate::device_trait::DeviceBase;

    use super::DeviceRTC;

    #[test]
    fn rtc_read() {
        let mut rtc = DeviceRTC::new();

        let low = rtc.do_read(0, 4);
        let high = rtc.do_read(4, 4);

        let t = (high << 32) + low;
        assert_eq!(t, rtc.rtc_time);
        println!("{t}");
        std::thread::sleep(std::time::Duration::from_millis(100));
        let low = rtc.do_read(0, 4);
        let high = rtc.do_read(4, 4);

        let t = (high << 32) + low;
        assert_eq!(t, rtc.rtc_time);
        println!("{t}");
    }
}
