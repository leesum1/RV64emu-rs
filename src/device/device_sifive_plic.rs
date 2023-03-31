use std::{cell::Cell, rc::Rc};

use bitfield_struct::bitfield;

use super::device_trait::DeviceBase;

const PLIC_PRIORITY_START: u64 = 0x0;
const PLIC_PRIORITY_END: u64 = 0x3FF; // 4×Interrupt ID

const PLIC_PENDING1: u64 = 0x1000;
const PLIC_PENDING2: u64 = 0x1004;
const PLIC_ENABLE1: u64 = 0x2000;
const PLIC_ENABLE2: u64 = 0x2004;
const PLIC_THRESHOLD: u64 = 0x20_0000;
const PLIC_CLAIM: u64 = 0x20_0004;

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
    pub fn is_mask_all(&mut self) -> bool {
        self.threshold() == 7
    }
    pub fn is_mask_none(&mut self) -> bool {
        self.threshold() == 0
    }
}

struct IrqClaim {
    val: u32,
}

impl IrqClaim {
    pub fn new() -> Self {
        IrqClaim { val: 0 }
    }
    pub fn set(&mut self, irq_id: u32) {
        self.val = irq_id;
    }
    pub fn get(&self) -> u32 {
        self.val
    }
}

pub struct DevicePlic {
    pub start: u64,
    pub len: u64,
    pub instance: SifvePlic,
    pub name: &'static str,
}

pub struct SifvePlic {
    irq_sources: Vec<IrqSource>,
    vec_irq_priority: Vec<IrqPriority>,
    irq_pending0_31: IrqPending,
    irq_pending32_63: IrqPending,
    irq_enable0_31: IrqEnable,
    irq_enable32_63: IrqEnable,
    irq_threshold: IrqThreshold,
}

impl SifvePlic {
    pub fn new() -> Self {
        SifvePlic {
            irq_sources: Vec::new(),
            vec_irq_priority: vec![IrqPriority::new(); 64],
            irq_pending0_31: IrqPending::new(),
            irq_pending32_63: IrqPending::new(),
            irq_enable0_31: IrqEnable::new(),
            irq_enable32_63: IrqEnable::new(),
            irq_threshold: IrqThreshold::new(),
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
    fn tick(&mut self) {
        for item in self.irq_sources.iter() {
            let pending_bit = item.pending.get();
            let irq_id = item.id;
            match irq_id {
                1..=31 => self.irq_pending0_31.set_bool(irq_id, pending_bit),
                32..=63 => self.irq_pending32_63.set_bool(irq_id, pending_bit),
                _ => panic!("irq_id:{} is too large", irq_id),
            }
        }
    }

    fn get_enable_bit(&self, irq_id: u32) -> bool {
        match irq_id {
            1..=31 => self.irq_enable0_31.get_bit(irq_id),
            32..=63 => self.irq_enable32_63.get_bit(irq_id),
            _ => false,
        }
    }

    fn set_pending_bit(&mut self, irq_id: u32, val: bool) {
        match irq_id {
            1..=31 => self.irq_pending0_31.set_bool(irq_id, val),
            32..=63 => self.irq_pending32_63.set_bool(irq_id, val),
            _ => panic!("irq_id:{} is too large", irq_id),
        }
    }

    pub fn plic_external_interrupt(&mut self) -> bool {
        // update pending bits
        self.tick();
        // check if there is any pending interrupt
        // if there is any pending interrupt, check if it is enabled and its priority is higher than the threshold
        // if so, return true
        self.irq_sources
            .iter()
            .filter(|item| item.pending.get())
            .any(|item| {
                let irq_id = item.id;
                let enable_bit = self.get_enable_bit(irq_id);
                let irq_priority = self.vec_irq_priority[irq_id as usize].get();
                let threshold = self.irq_threshold.threshold();
                enable_bit && irq_priority > threshold
            })
    }

    // 1. Get the highest priority pending interrupt, and clear the pending bit.
    // 2. Return the interrupt ID.
    fn claim_read(&mut self) -> u32 {
        let mut irq_id = 0;
        let mut irq_priority = 0;
        let mut irq_pendding = Rc::new(Cell::new(false));
        self.irq_sources.iter().for_each(|item| {
            let pending_bit = item.pending.get();
            let enable_bit = self.get_enable_bit(item.id);
            if pending_bit & enable_bit {
                let irq_priority_tmp = self.vec_irq_priority[item.id as usize].get();
                if irq_priority_tmp > irq_priority {
                    irq_priority = irq_priority_tmp;
                    irq_pendding = Rc::clone(&item.pending);
                    irq_id = item.id;
                }
                // Ties between global interrupts of the same priority are broken by the
                // Interrupt ID; interrupts with the lowest ID have the highest effective priority
                else if irq_priority_tmp == irq_priority && item.id < irq_id {
                    irq_pendding = Rc::clone(&item.pending);
                    irq_id = item.id;
                }
            }
        });
        irq_pendding.set(false);
        irq_id
    }
    fn claim_write(&mut self, irq_id: u32) {
        self.set_pending_bit(irq_id, false);
    }

    fn set_enable_bit(&mut self, irq_id: u32, val: bool) {
        match irq_id {
            1..=31 => self.irq_enable0_31.set_bool(irq_id, val),
            32..=63 => self.irq_enable32_63.set_bool(irq_id, val),
            _ => panic!("irq_id:{} is too large", irq_id),
        }
    }
}

impl DeviceBase for SifvePlic {
    fn do_read(&mut self, addr: u64, len: usize) -> u64 {
        assert_eq!(len, 4, "plic read len:{}", len);
        match addr {
            PLIC_PRIORITY_START..=PLIC_PRIORITY_END => {
                let irq_id = (addr - PLIC_PRIORITY_START) / 4;
                let irq_priority = self.vec_irq_priority[irq_id as usize];
                irq_priority.get().into()
            }
            PLIC_PENDING1 => self.irq_pending0_31.get_all().into(),
            PLIC_PENDING2 => self.irq_pending32_63.get_all().into(),
            PLIC_ENABLE1 => self.irq_enable0_31.get_all().into(),
            PLIC_ENABLE2 => self.irq_enable32_63.get_all().into(),
            PLIC_THRESHOLD => self.irq_threshold.threshold().into(),
            PLIC_CLAIM => self.claim_read().into(),
            _ => {
                panic!("plic read addr:0x{:x} len:{}", addr, len);
            }
        }
    }
    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        assert_eq!(len, 4, "plic write len:{}", len);
        match addr {
            PLIC_PRIORITY_START..=PLIC_PRIORITY_END => {
                let irq_id = (addr - PLIC_PRIORITY_START) / 4;
                if let Some(irq_priority) = self.vec_irq_priority.get_mut(irq_id as usize) {
                    irq_priority.set(data as u8)
                }
            }
            PLIC_ENABLE1 => self.irq_enable0_31.set_all(data as u32),
            PLIC_ENABLE2 => self.irq_enable32_63.set_all(data as u32),
            PLIC_THRESHOLD => self.irq_threshold.set_threshold(data as u8),
            PLIC_CLAIM => self.claim_write(data as u32),
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

#[cfg(test)]
mod plic_test {
    use std::{cell::Cell, rc::Rc};

    use crate::device::{
        device_sifive_plic::{PLIC_PRIORITY_START, PLIC_THRESHOLD},
        device_trait::DeviceBase,
    };

    use super::SifvePlic;

    #[test]
    fn test1() {
        let uart_irq = Rc::new(Cell::new(false));
        let iic_irq = Rc::new(Cell::new(false));
        let mut plic = SifvePlic::new();
        plic.register_irq_source(1, Rc::clone(&uart_irq));
        plic.register_irq_source(2, Rc::clone(&iic_irq));

        plic.set_enable_bit(1, true);
        plic.do_write(PLIC_THRESHOLD, 0, 4);
        plic.do_write(PLIC_PRIORITY_START + 4 * 1, 2, 4);

        uart_irq.set(false);
        assert_eq!(plic.plic_external_interrupt(), false);
    }
}
