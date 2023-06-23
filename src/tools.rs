use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crossbeam_queue::ArrayQueue;
use crossbeam_queue::SegQueue;

pub type FifoUnbounded<T> = Arc<SegQueue<T>>;
pub type Fifobounded<T> = Arc<ArrayQueue<T>>;
pub type RVmutex<T> = Rc<RefCell<T>>;
