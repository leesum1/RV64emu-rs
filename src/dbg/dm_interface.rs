pub trait DebugModuleSlave {
    fn hart_id(&mut self) -> usize {
        // pub const CSR_MHARTID: u16 = 0xf14;
        self.read_csr(0xf14) as usize
    }

    // read and write gpr
    fn read_gpr(&mut self, regno: usize) -> u64;
    fn write_gpr(&mut self, regno: usize, value: u64);

    // pysically memory access
    fn read_memory(&mut self, address: u64, length: usize) -> Option<u64>;
    fn write_memory(&mut self, address: u64, length: usize, value: u64) -> Option<u64>;

    // read and write csr
    fn read_csr(&mut self, csr_addr: usize) -> u64;
    fn write_csr(&mut self, csr_addr: usize, value: u64);

    // halt and resume
    fn set_haltreq(&mut self, val: bool);
    fn resumereq(&mut self);
    fn halted(&mut self) -> bool;
    fn running(&mut self) -> bool {
        !self.halted()
    }
    fn resume_ack(&mut self) -> bool;
    fn set_reset_req(&mut self, val: bool);
    fn havereset(&mut self) -> bool;
    fn clear_havereset(&mut self);
}
