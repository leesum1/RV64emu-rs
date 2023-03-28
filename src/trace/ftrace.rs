use std::{fs::File, io::Write};

pub struct Ftrace {
    log_file: File,
    cur_deepth: usize,
}

impl Ftrace {
    pub fn new() -> Self {
        let fd = File::create("/tmp/rv64emu_ftrace_logs").unwrap();
        Ftrace {
            log_file: fd,
            cur_deepth: 0,
        }
    }

    pub fn call_record(&mut self, inst_pc: u64, pc: u64) {
        let call_str = format!(
            "pc:{:08x},deep:{} Call  --->0x{:08x}\n",
            inst_pc, self.cur_deepth, pc
        );
        self.log_file.write_all(call_str.as_bytes()).unwrap();
        self.cur_deepth += 1;
    }

    pub fn ret_record(&mut self, inst_pc: u64, pc: u64) {
        self.cur_deepth = self.cur_deepth.wrapping_sub(1);
        let ret_str = format!(
            "pc:{:08x},deep:{} Return<---0x{:08x}\n",
            inst_pc, self.cur_deepth, pc
        );
        self.log_file.write_all(ret_str.as_bytes()).unwrap();
    }
}
