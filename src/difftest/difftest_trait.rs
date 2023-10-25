/**
 * 1. get gpr value
 * 2. get csr value
 * 3. get memory value
 * 4. set gpr value
 * 5. set csr value
 * 6. set memory value
 *
*/

// pub enum DifftestDirection {
//     DIFFTEST_TO_REF = 0,
//     REF_TO_DIFFTEST = 1,
// }
pub trait Difftest {
    fn set_reg(&mut self, idx: usize, val: u64);
    fn get_reg(&self, idx: usize) -> u64;
    fn get_pc(&self) -> u64;
    fn set_pc(&mut self, pc: u64);
    fn raise_intr(&mut self, irq: u64);
    fn set_csr(&mut self, csr: u64, val: u64);
    fn get_csr(&mut self, csr: u64) -> u64;
    fn set_mem(&mut self, paddr: u64, data: u64, len: usize);
    fn get_mem(&self, paddr: u64, len: usize) -> u64;
}
