use std::{cell::Cell, cmp::Ordering, rc::Rc};

use bitfield_struct::bitfield;
use log::warn;

use crate::rv64core::csr_regs_define::XipIn;

use super::device_trait::DeviceBase;

/* ref spike plic */
const _PLIC_MAX_CONTEXTS: usize = 15872;
/*
 * The PLIC consists of memory-mapped control registers, with a memory map
 * as follows:
 *
 * base + 0x000000: Reserved (interrupt source 0 does not exist)
 * base + 0x000004: Interrupt source 1 priority
 * base + 0x000008: Interrupt source 2 priority
 * ...
 * base + 0x000FFC: Interrupt source 1023 priority
 * base + 0x001000: Pending 0
 * base + 0x001FFF: Pending
 * base + 0x002000: Enable bits for sources 0-31 on context 0
 * base + 0x002004: Enable bits for sources 32-63 on context 0
 * ...
 * base + 0x0020FC: Enable bits for sources 992-1023 on context 0
 * base + 0x002080: Enable bits for sources 0-31 on context 1
 * ...
 * base + 0x002100: Enable bits for sources 0-31 on context 2
 * ...
 * base + 0x1F1F80: Enable bits for sources 992-1023 on context 15871
 * base + 0x1F1F84: Reserved
 * ...		    (higher context IDs would fit here, but wouldn't fit
 *		     inside the per-context priority vector)
 * base + 0x1FFFFC: Reserved
 * base + 0x200000: Priority threshold for context 0
 * base + 0x200004: Claim/complete for context 0
 * base + 0x200008: Reserved
 * ...
 * base + 0x200FFC: Reserved
 * base + 0x201000: Priority threshold for context 1
 * base + 0x201004: Claim/complete for context 1
 * ...
 * base + 0xFFE000: Priority threshold for context 15871
 * base + 0xFFE004: Claim/complete for context 15871
 * base + 0xFFE008: Reserved
 * ...
 * base + 0xFFFFFC: Reserved
 */

/* Each interrupt source has a priority register associated with it. */
const PRIORITY_BASE: u64 = 0;
const _PRIORITY_PER_ID: u64 = 4;
const PRIORITY_END: u64 = PENDING_BASE - 1;

/* Each interrupt source has a pending bit associated with it. */
const PENDING_BASE: u64 = 0x1000;
const PENDING_END: u64 = ENABLE_BASE - 1;
/*
 * Each hart context has a vector of interupt enable bits associated with it.
 * There's one bit for each interrupt source.
 */
const ENABLE_BASE: u64 = 0x2000;
const ENABLE_PER_HART: u64 = 0x80;
const ENABLE_END: u64 = CONTEXT_BASE - 1;
/*
 * Each hart context has a set of control registers associated with it.  Right
 * now there's only two: a source priority threshold over which the hart will
 * take an interrupt, and a register to claim interrupts.
 */
const CONTEXT_BASE: u64 = 0x200000;
const CONTEXT_PER_HART: u64 = 0x1000;
const CONTEXT_THRESHOLD: u64 = 0;
const CONTEXT_CLAIM: u64 = 4;

const CONTEXT_END: u64 = REG_SIZE - 1;
const REG_SIZE: u64 = 0x1000000;

pub const SIFIVE_UART_IRQ: u32 = 10;

struct IrqSource {
    id: u32,
    pending: Rc<Cell<bool>>,
}

// PLIC Interrupt Priority Register (priority)
// Base Address 0x0C00_0000 + 4×Interrupt ID
#[bitfield(u32)]
struct IrqPriority {
    #[bits(3)]
    pub priority: u8,
    #[bits(29)]
    pub reserved: u32,
}
impl IrqPriority {
    pub fn get(&self) -> u8 {
        self.priority()
    }
    pub fn set(&mut self, priority: u8) {
        self.set_priority(priority)
    }
}

#[derive(Clone, Copy)]
struct IrqPending {
    val: u32,
}
impl IrqPending {
    pub fn new() -> Self {
        IrqPending { val: 0 }
    }
    pub fn set_bool(&mut self, irq_id: u32, val: bool) {
        if val {
            self.set_bit(irq_id);
        } else {
            self.clear_bit(irq_id);
        }
    }
    // set the irq_id bit to 1
    pub fn set_bit(&mut self, irq_id: u32) {
        self.val |= 1 << irq_id;
    }
    // set the irq_id bit to 0
    pub fn clear_bit(&mut self, irq_id: u32) {
        self.val &= !(1 << irq_id);
    }
    // get the irq_id bit
    pub fn get_bit(&self, irq_id: u32) -> bool {
        (self.val & (1 << irq_id)) != 0
    }

    pub fn set_all(&mut self, val: u32) {
        self.val = val;
    }
    pub fn get_all(&self) -> u32 {
        self.val
    }
}
// The enables registers are accessed as a contiguous array of 2×32-bit words,
// packed the same way as the pending bits.
type IrqEnable = IrqPending;

#[bitfield(u32)]
struct IrqThreshold {
    #[bits(3)]
    pub threshold: u8,
    #[bits(29)]
    pub reserved: u32,
}

impl IrqThreshold {
    pub fn get_all(&self) -> u32 {
        self.threshold() as u32
    }
    pub fn set_all(&mut self, val: u32) {
        self.set_threshold(val as u8)
    }
}

pub struct DevicePlic {
    pub start: u64,
    pub len: u64,
    pub instance: SifvePlic,
    pub name: &'static str,
}

struct PlicContext {
    mmode: bool,
    xip: Rc<Cell<XipIn>>,
    threshold: IrqThreshold,
    enable: [IrqEnable; 2], // 0: 0-31, 1: 32-63
    claim: u32,
}
impl PlicContext {
    pub fn new(xip_share: Rc<Cell<XipIn>>, mmode: bool) -> Self {
        PlicContext {
            threshold: IrqThreshold::new(),
            enable: [IrqEnable::new(); 2],
            claim: 0,
            mmode,
            xip: xip_share,
        }
    }
    pub fn get_enable_by_id(&self, irq_id: u32) -> bool {
        self.enable[irq_id as usize / 32].get_bit(irq_id % 32)
    }

    pub fn update_xip(&self, level: bool) {
        let mut xip = self.xip.get();
        match self.mmode {
            true => xip.set_meip(level),
            false => xip.set_seip(level),
        };
        self.xip.set(xip);
    }
}
pub struct SifvePlic {
    irq_sources: Vec<IrqSource>,
    // regs
    vec_irq_priority: Vec<IrqPriority>,
    irq_pending: [IrqPending; 2], // 0: 0-31, 1: 32-63
    claimed: [bool; 64],
    context: Vec<PlicContext>,
}

impl SifvePlic {
    pub fn new() -> Self {
        SifvePlic {
            irq_sources: Vec::new(),
            vec_irq_priority: vec![IrqPriority::new(); 64],
            irq_pending: [IrqPending::new(); 2],
            claimed: [false; 64],
            context: Vec::new(),
        }
    }
    pub fn register_irq_source(&mut self, irq_id: u32, irq_pending: Rc<Cell<bool>>) {
        assert!(irq_id < 64, "irq_id:{} is too large", irq_id);
        // Check if the irq_id is already registered, if so, panic
        if self.irq_sources.iter().any(|item| item.id == irq_id) {
            panic!("irq_id:{} is already registered", irq_id);
        };
        // Register the new irq_source
        self.irq_sources.push(IrqSource {
            id: irq_id,
            pending: irq_pending,
        });
    }
    pub fn add_context(&mut self, xip_share: Rc<Cell<XipIn>>, mmode: bool) {
        self.context.push(PlicContext::new(xip_share, mmode));
    }
    pub fn tick(&mut self) {
        // Update the pending bits
        for item in self.irq_sources.iter() {
            let pending_bit = item.pending.get();
            let irq_id = item.id;
            self.irq_pending[irq_id as usize / 32].set_bool(irq_id % 32, pending_bit);
        }
        // Update the xip
        for c in &self.context {
            let int_flag = self.irq_sources.iter().any(|item| {
                let irq_id = item.id;
                let pending_bit = item.pending.get();
                let enable_bit = c.get_enable_by_id(irq_id);
                let priority = self.vec_irq_priority[irq_id as usize].get();
                let threshold = c.threshold.get_all() as u8;
                pending_bit && enable_bit && priority > threshold
            });
            // if int_flag {
            //     warn!("int_flag is true");
            // }
            c.update_xip(int_flag);
        }
    }

    fn priority_read(&self, offset: u32) -> u32 {
        let idx_word = (offset >> 2) as usize;
        self.vec_irq_priority[idx_word].get() as u32
    }
    fn priority_write(&mut self, offset: u32, val: u32) {
        let idx_word = (offset >> 2) as usize;
        self.vec_irq_priority[idx_word].set(val as u8);
    }

    fn pending_read(&self, offset: u32) -> u32 {
        let idx_word = (offset >> 2) as usize;
        self.irq_pending[idx_word].get_all()
    }
    // pending bits are read-only
    fn pending_write(&mut self, _irq_id: u32, _val: u32) {
        warn!("pending bits are read-only");
    }

    fn context_enbale_read(&self, offset: u32) -> u32 {
        let context_idx = (offset / ENABLE_PER_HART as u32) as usize;
        let idx_word = ((offset % ENABLE_PER_HART as u32) >> 2) as usize;
        self.context[context_idx].enable[idx_word].get_all()
    }
    fn context_enbale_write(&mut self, offset: u32, val: u32) {
        let context_idx = (offset / ENABLE_PER_HART as u32) as usize;
        let idx_word = ((offset % ENABLE_PER_HART as u32) >> 2) as usize;
        // The first bit of the first word is reserved and must be zero
        let val = if idx_word == 0 { val & !1 } else { val };
        self.context[context_idx].enable[idx_word].set_all(val);
    }

    fn context_read(&mut self, offset: u32) -> u32 {
        let context_idx = (offset / CONTEXT_PER_HART as u32) as usize;
        let context_offset = (offset % CONTEXT_PER_HART as u32) as u64;
        match context_offset {
            CONTEXT_THRESHOLD => self.context[context_idx].threshold.get_all(),
            CONTEXT_CLAIM => {
                // debug!("context_read(context_idx:{})", context_idx);
                // println!("context_read(context_idx:{})", context_idx);

                self.context_claim(context_idx)
            }
            _ => panic!("context_read invalid offset:{}", context_offset),
        }
    }
    fn context_write(&mut self, offset: u32, val: u32) {
        let context_idx = (offset / CONTEXT_PER_HART as u32) as usize;
        let context_offset = (offset % CONTEXT_PER_HART as u32) as u64;

        match context_offset {
            CONTEXT_THRESHOLD => self.context[context_idx].threshold.set_all(val),
            CONTEXT_CLAIM => {
                // debug!("context_write(context_idx:{}, val:{})", context_idx, val);
                self.context_complete(context_idx, val)
            }
            _ => panic!("context_write invalid offset:{}", context_offset),
        }
    }

    fn context_claim(&mut self, context_idx: usize) -> u32 {
        // 1. Get the highest priority pending interrupt, and clear the pending bit.
        // 2. Return the interrupt ID
        let mut irq_id = 0;
        let mut irq_priority = 0;
        let mut irq_pendding = Rc::new(Cell::new(false));
        let c = self.context.get_mut(context_idx).unwrap();

        self.irq_sources
            .iter()
            .filter(|item| item.pending.get() && c.get_enable_by_id(item.id))
            .for_each(|item| {
                let priority_tmp = self.vec_irq_priority[item.id as usize].get();
                match priority_tmp.cmp(&irq_priority) {
                    Ordering::Greater => {
                        irq_id = item.id;
                        irq_priority = priority_tmp;
                        irq_pendding = item.pending.clone();
                    }
                    Ordering::Equal => {
                        // If two interrupts have the same priority, the one with the lower interrupt ID is selected.
                        if item.id < irq_id {
                            irq_id = item.id;
                            irq_priority = priority_tmp;
                            irq_pendding = item.pending.clone();
                        }
                    }
                    Ordering::Less => {}
                };
            });

        match self.claimed[irq_id as usize] {
            true => 0,
            false => {
                // debug!("context_claim(context_idx:{}),id:{}", context_idx,irq_id);
                irq_pendding.set(false);
                self.claimed[irq_id as usize] = true;
                c.claim = irq_id;
                irq_id
            }
        }
        // warn!(
        //     "context_claim(context_idx:{}, irq_id:{})",
        //     context_idx, irq_id
        // );
    }

    fn context_complete(&mut self, _context_idx: usize, val: u32) {
        self.claimed[val as usize] = false;
    }
}

impl Default for SifvePlic {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceBase for SifvePlic {
    fn do_read(&mut self, addr: u64, len: usize) -> u64 {
        assert_eq!(len, 4, "plic write len:{}", len);
        // debug!("plic read addr:0x{:x} len:{}", addr, len);
        match addr {
            PRIORITY_BASE..=PRIORITY_END => {
                let offset = (addr - PRIORITY_BASE) as u32;
                self.priority_read(offset) as u64
            }
            PENDING_BASE..=PENDING_END => {
                let offset = (addr - PENDING_BASE) as u32;
                self.pending_read(offset) as u64
            }
            ENABLE_BASE..=ENABLE_END => {
                let offset = (addr - ENABLE_BASE) as u32;
                self.context_enbale_read(offset) as u64
            }
            CONTEXT_BASE..=CONTEXT_END => {
                let offset = (addr - CONTEXT_BASE) as u32;
                self.context_read(offset) as u64
            }
            _ => {
                panic!("plic read addr:0x{:x} len:{}", addr, len)
            }
        }
    }
    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        assert_eq!(len, 4, "plic write len:{}", len);
        match addr {
            PRIORITY_BASE..=PRIORITY_END => {
                let offset = (addr - PRIORITY_BASE) as u32;
                self.priority_write(offset, data as u32);
            }
            PENDING_BASE..=PENDING_END => {
                let offset = (addr - PENDING_BASE) as u32;
                self.pending_write(offset, data as u32);
            }
            ENABLE_BASE..=ENABLE_END => {
                let offset = (addr - ENABLE_BASE) as u32;
                self.context_enbale_write(offset, data as u32);
            }
            CONTEXT_BASE..=CONTEXT_END => {
                let offset = (addr - CONTEXT_BASE) as u32;
                self.context_write(offset, data as u32);
            }
            _ => {
                panic!("plic write addr:0x{:x} data:0x{:x} len:{}", addr, data, len);
            }
        }
        0
    }
    fn get_name(&self) -> &'static str {
        "PLIC"
    }
}
