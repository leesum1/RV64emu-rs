

use strum_macros::{Display, EnumString, FromRepr, IntoStaticStr};

const INTERRUPT_BIT: u64 = 0x8000000000000000_u64;

#[allow(dead_code)]
#[repr(u64)]
#[derive(EnumString, FromRepr, IntoStaticStr, Display, Debug, PartialEq, Clone, Copy)]
pub enum TrapType {
    // InstructionAddressMisaligned = 0,
    // InstructionAccessFault = 1,
    // IllegalInstruction = 2,
    // Breakpoint = 3,
    // LoadAddressMisaligned = 4,
    // LoadAccessFault = 5,
    // StoreAddressMisaligned = 6,
    // StoreAccessFault = 7,
    // EnvironmentCallFromUMode = 8,
    // EnvironmentCallFromSMode = 9,
    // EnvironmentCallFromMMode = 11,
    // InstructionPageFault = 12,
    // LoadPageFault = 13,
    // StorePageFault = 15,
    // UserSoftwareInterrupt = INTERRUPT_BIT,
    // SupervisorSoftwareInterrupt = INTERRUPT_BIT + 1,
    // MachineSoftwareInterrupt = INTERRUPT_BIT + 3,
    // UserTimerInterrupt = INTERRUPT_BIT + 4,
    // SupervisorTimerInterrupt = INTERRUPT_BIT + 5,
    // MachineTimerInterrupt = INTERRUPT_BIT + 7,
    // UserExternalInterrupt = INTERRUPT_BIT + 8,
    // SupervisorExternalInterrupt = INTERRUPT_BIT + 9,
    // MachineExternalInterrupt = INTERRUPT_BIT + 11,
    InstructionAddressMisaligned(u64),
    InstructionAccessFault(u64),
    IllegalInstruction(u64),
    Breakpoint,
    LoadAddressMisaligned(u64),
    LoadAccessFault(u64),
    StoreAddressMisaligned(u64),
    StoreAccessFault(u64),
    EnvironmentCallFromUMode = 8,
    EnvironmentCallFromSMode = 9,
    EnvironmentCallFromMMode = 11,
    InstructionPageFault(u64),
    LoadPageFault(u64),
    StorePageFault(u64),
    UserSoftwareInterrupt,
    SupervisorSoftwareInterrupt,
    MachineSoftwareInterrupt,
    UserTimerInterrupt,
    SupervisorTimerInterrupt,
    MachineTimerInterrupt,
    UserExternalInterrupt,
    SupervisorExternalInterrupt,
    MachineExternalInterrupt,
}

impl TrapType {
    pub fn idx(&self) -> u64 {
        match self {
            TrapType::InstructionAddressMisaligned(_) => 0,
            TrapType::InstructionAccessFault(_) => 1,
            TrapType::IllegalInstruction(_) => 2,
            TrapType::Breakpoint => 3,
            TrapType::LoadAddressMisaligned(_) => 4,
            TrapType::LoadAccessFault(_) => 5,
            TrapType::StoreAddressMisaligned(_) => 6,
            TrapType::StoreAccessFault(_) => 7,
            TrapType::EnvironmentCallFromUMode => 8,
            TrapType::EnvironmentCallFromSMode => 9,
            TrapType::EnvironmentCallFromMMode => 11,
            TrapType::InstructionPageFault(_) => 12,
            TrapType::LoadPageFault(_) => 13,
            TrapType::StorePageFault(_) => 15,
            TrapType::UserSoftwareInterrupt => INTERRUPT_BIT + 0,
            TrapType::SupervisorSoftwareInterrupt => INTERRUPT_BIT + 1,
            TrapType::MachineSoftwareInterrupt => INTERRUPT_BIT + 3,
            TrapType::UserTimerInterrupt => INTERRUPT_BIT + 4,
            TrapType::SupervisorTimerInterrupt => INTERRUPT_BIT + 5,
            TrapType::MachineTimerInterrupt => INTERRUPT_BIT + 7,
            TrapType::UserExternalInterrupt => INTERRUPT_BIT + 8,
            TrapType::SupervisorExternalInterrupt => INTERRUPT_BIT + 9,
            TrapType::MachineExternalInterrupt => INTERRUPT_BIT + 11,
        }
    }

    pub fn is_interupt(&self) -> bool {
        ((self.idx()) & INTERRUPT_BIT) != 0
    }
    pub fn get_irq_num(&self) -> u64 {
        assert!(self.is_interupt());
        (self.idx()) & (!INTERRUPT_BIT)
    }

    pub fn get_exception_num(&self) -> u64 {
        assert!(!self.is_interupt());
        self.idx() as u64
    }

    pub fn get_tval(&self) -> u64 {
        match self {
            TrapType::LoadPageFault(val)
            | TrapType::StorePageFault(val)
            | TrapType::StoreAccessFault(val)
            | TrapType::LoadAccessFault(val)
            | TrapType::LoadAddressMisaligned(val)
            | TrapType::StoreAddressMisaligned(val)
            | TrapType::InstructionAccessFault(val)
            | TrapType::InstructionPageFault(val)
            | TrapType::IllegalInstruction(val) => *val,
            _ => 0,
        }
    }
}

// #[test]
// fn trap_type_test1(){
//     let x = TrapType::MachineTimerInterrupt;
//     assert!(x.is_interupt());
//     assert_eq!(x.get_irq_num(),7);

//     let x = TrapType::MachineExternalInterrupt;
//     assert!(x.is_interupt());
//     assert_eq!(x.get_irq_num(),11);

//     let x = TrapType::StoreAddressMisaligned;
//     assert!(!x.is_interupt());
// }
