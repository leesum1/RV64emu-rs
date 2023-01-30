

pub trait DeviceBase {

    fn do_read(&mut self, addr: u64, len: usize) -> u64;
    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64;
    fn do_update(&mut self) {
        println!("do_update");
    }
}
