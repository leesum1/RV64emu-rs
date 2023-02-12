use strum_macros::{Display, EnumString, FromRepr, IntoStaticStr};

const INTERRUPT_BIT: u64 = 0x8000000000000000_u64;

#[allow(dead_code)]
#[repr(u64)]
#[derive(EnumString, FromRepr, IntoStaticStr, Display, Debug, PartialEq, Clone, Copy)]
pub enum TrapType {
    InstructionAddressMisaligned = 0,
    InstructionAccessFault = 1,
    IllegalInstruction = 2,
    Breakpoint = 3,
    LoadAddressMisaligned = 4,
    LoadAccessFault = 5,
    StoreAddressMisaligned = 6,
    StoreAccessFault = 7,
    EnvironmentCallFromUMode = 8,
    EnvironmentCallFromSMode = 9,
    EnvironmentCallFromMMode = 11,
    InstructionPageFault = 12,
    LoadPageFault = 13,
    StorePageFault = 15,
    UserSoftwareInterrupt = INTERRUPT_BIT,
    SupervisorSoftwareInterrupt = INTERRUPT_BIT + 1,
    MachineSoftwareInterrupt = INTERRUPT_BIT + 3,
    UserTimerInterrupt = INTERRUPT_BIT + 4,
    SupervisorTimerInterrupt = INTERRUPT_BIT + 5,
    MachineTimerInterrupt = INTERRUPT_BIT + 7,
    UserExternalInterrupt = INTERRUPT_BIT + 8,
    SupervisorExternalInterrupt = INTERRUPT_BIT + 9,
    MachineExternalInterrupt = INTERRUPT_BIT + 11,
}
