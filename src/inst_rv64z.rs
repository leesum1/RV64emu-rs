use crate::inst_base::*;

pub const INSTRUCTIONS_Z: [Instruction; 3] = [
    Instruction {
        mask: MASK_EBREAK,
        match_data: MATCH_EBREAK,
        name: "EBREAK",
        operation: |cpu, inst, pc| {
            cpu.halt();
            Ok(())
        },
    },
    Instruction {
        mask: MASK_FENCE_I,
        match_data: MATCH_FENCE_I,
        name: "FENCE_I",
        operation: |cpu, inst, pc| {

            Ok(())
        },
    },
    Instruction {
        mask: MASK_FENCE,
        match_data: MATCH_FENCE,
        name: "FENCE",
        operation: |cpu, inst, pc| {

            Ok(())
        },
    },
];
