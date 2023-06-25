use core::cell::Cell;
use core::cell::RefCell;

extern crate alloc;
use alloc::rc::Rc;
use alloc::sync::Arc;
use crossbeam_queue::ArrayQueue;
use crossbeam_queue::SegQueue;

pub type FifoUnbounded<T> = Arc<SegQueue<T>>;
pub type Fifobounded<T> = Arc<ArrayQueue<T>>;
pub type RcRefCell<T> = Rc<RefCell<T>>;
pub type RcCell<T> = Rc<Cell<T>>;

pub fn fifo_unbounded_new<T>() -> FifoUnbounded<T> {
    Arc::new(SegQueue::new())
}
pub fn fifo_bounded_new<T>(size: usize) -> Fifobounded<T> {
    Arc::new(ArrayQueue::new(size))
}

pub fn rc_refcell_new<T>(t: T) -> RcRefCell<T> {
    Rc::new(RefCell::new(t))
}
pub fn rc_cell_new<T>(t: T) -> RcCell<T> {
    Rc::new(Cell::new(t))
}
pub fn sign_extended(x: isize, nbits: u32) -> isize {
    let notherbits = isize::BITS - nbits;
    x.wrapping_shl(notherbits).wrapping_shr(notherbits)
}

pub fn check_area(start: u64, len: u64, addr: u64) -> bool {
    (start..(start + len)).contains(&addr)
}

pub fn check_aligned(addr: u64, len: usize) -> bool {
    // assert!(addr & (len - 1) == 0, "bus address not aligned");
    addr & (len as u64 - 1) == 0
}
