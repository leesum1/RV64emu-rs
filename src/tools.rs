use core::cell::Cell;
use core::cell::RefCell;

extern crate alloc;
use alloc::rc::Rc;
use alloc::sync::Arc;
use crossbeam_queue::ArrayQueue;
use crossbeam_queue::SegQueue;

pub type FifoUnbounded<T> = Arc<SegQueue<T>>;
pub type Fifobounded<T> = Arc<ArrayQueue<T>>;
pub type RVmutex<T> = Rc<RefCell<T>>;
pub type CsrShare<T> = Rc<Cell<T>>;

pub fn fifo_unbounded_new<T>() -> FifoUnbounded<T> {
    Arc::new(SegQueue::new())
}
pub fn fifo_bounded_new<T>(size: usize) -> Fifobounded<T> {
    Arc::new(ArrayQueue::new(size))
}
