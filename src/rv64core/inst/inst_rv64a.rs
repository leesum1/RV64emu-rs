use crate::rv64core::inst::inst_base::*;

pub struct LrScReservation {
    pub val: u64,
}

impl LrScReservation {
    pub fn new() -> Self {
        LrScReservation { val: u64::MAX }
    }

    pub fn check_and_clear(&mut self, addr: u64) -> bool {
        let ret = self.val == addr;
        self.clear();
        ret
    }
    pub fn set(&mut self, addr: u64) {
        self.val = addr
    }
    pub fn clear(&mut self) {
        self.val = u64::MAX
    }
}

impl Default for LrScReservation {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(unused_variables)]
pub const INSTRUCTIONS_A: &[Instruction] = &[
    Instruction {
        mask: MASK_LR_W,
        match_data: MATCH_LR_W,
        name: "LR_D",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let r_data = match cpu.read(rs1_data, 4, AccessType::Load(rs1_data)) {
                Ok(data) => data as i32 as i64,
                Err(trap_type) => return Err(trap_type),
            };

            cpu.lr_sc_reservation_set(rs1_data);
            cpu.gpr.write(f.rd, r_data as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_LR_D,
        match_data: MATCH_LR_D,
        name: "LR_D",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let r_data = match cpu.read(rs1_data, 8, AccessType::Load(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            cpu.lr_sc_reservation_set(rs1_data);

            cpu.gpr.write(f.rd, r_data);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SC_W,
        match_data: MATCH_SC_W,
        name: "SC_W",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);

            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2);

            if cpu.lr_sc_reservation_check_and_clear(rs1_data) {
                match cpu.write(rs1_data, rs2_data, 4, AccessType::Store(rs1_data)) {
                    Ok(_) => {}
                    Err(trap_type) => return Err(trap_type),
                }
                cpu.gpr.write(f.rd, 0);
            } else {
                cpu.gpr.write(f.rd, 1);
            }
            Ok(())
        },
    },
    Instruction {
        mask: MASK_SC_D,
        match_data: MATCH_SC_D,
        name: "SC_D",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);

            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2);

            if cpu.lr_sc_reservation_check_and_clear(rs1_data) {
                // todo!
                match cpu.write(rs1_data, rs2_data, 8, AccessType::Store(rs1_data)) {
                    Ok(_) => {}
                    Err(trap_type) => return Err(trap_type),
                };
                cpu.gpr.write(f.rd, 0);
            } else {
                cpu.gpr.write(f.rd, 1);
            }
            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOSWAP_W,
        match_data: MATCH_AMOSWAP_W,
        name: "AMOSWAP_W",
        operation: |cpu, inst, pc| {
            // Atomic Memory Operation: Swap Word. R-type, RV32A and RV64A.
            // Atomically, let t be the value of the memory word at address x[rs1], then set that memory
            // word to x[rs2]. Set x[rd] to the sign extension of t.
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2);

            let tmp = match cpu.read(rs1_data, 4, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            // no err happenes here
            cpu.write(rs1_data, rs2_data, 4, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp as u32 as i32 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOSWAP_D,
        match_data: MATCH_AMOSWAP_D,
        name: "AMOSWAP_D",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2);

            let tmp = match cpu.read(rs1_data, 8, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            // no err happenes here
            cpu.write(rs1_data, rs2_data, 8, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOXOR_W,
        match_data: MATCH_AMOXOR_W,
        name: "AMOXOR_W",
        operation: |cpu, inst, pc| {
            /*Atomic Memory Operation: XOR Word. R-type, RV32A and RV64A.
              Atomically, let t be the value of the memory word at address x[rs1], then set that memory
              word to the bitwise XOR of t and x[rs2]. Set x[rd] to the sign extension of t.
            */
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2);

            let tmp = match cpu.read(rs1_data, 4, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            // no err happenes here
            cpu.write(rs1_data, tmp ^ rs2_data, 4, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp as u32 as i32 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOXOR_D,
        match_data: MATCH_AMOXOR_D,
        name: "AMOXOR_D",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2);

            let tmp = match cpu.read(rs1_data, 8, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            // no err happenes here
            cpu.write(rs1_data, tmp ^ rs2_data, 8, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOOR_W,
        match_data: MATCH_AMOOR_W,
        name: "AMOOR_W",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2);

            let tmp = match cpu.read(rs1_data, 4, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            // no err happenes here
            cpu.write(rs1_data, tmp | rs2_data, 4, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp as u32 as i32 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOOR_D,
        match_data: MATCH_AMOOR_D,
        name: "AMOOR_D",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2);

            let tmp = match cpu.read(rs1_data, 8, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            // no err happenes here
            cpu.write(rs1_data, tmp | rs2_data, 8, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOMINU_W,
        match_data: MATCH_AMOMINU_W,
        name: "AMOMINU_W",
        operation: |cpu, inst, pc| {
            // Atomic Memory Operation: Minimum Word, Unsigned. R-type, RV32A and RV64A.
            // Atomically, let t be the value of the memory word at address x[rs1], then set that memory
            // word to the smaller of t and x[rs2], using an unsigned comparison. Set x[rd] to the sign
            // extension of t.
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2) as u32;

            let tmp = match cpu.read(rs1_data, 4, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            let amo_write = (tmp as u32).min(rs2_data);
            // no err happenes here
            cpu.write(rs1_data, amo_write as u64, 4, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp as i32 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOMINU_D,
        match_data: MATCH_AMOMINU_D,
        name: "AMOMINU_D",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2);

            let tmp = match cpu.read(rs1_data, 8, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            let amo_write = tmp.min(rs2_data);
            // no err happenes here
            cpu.write(rs1_data, amo_write, 8, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOMIN_W,
        match_data: MATCH_AMOMIN_W,
        name: "AMOMIN_W",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2) as i32;

            let tmp = match cpu.read(rs1_data, 4, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            let amo_write = (tmp as i32).min(rs2_data);
            // no err happenes here
            cpu.write(rs1_data, amo_write as u64, 4, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp as i32 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOMIN_D,
        match_data: MATCH_AMOMIN_D,
        name: "AMOMIN_D",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2) as i64;

            let tmp = match cpu.read(rs1_data, 8, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            let amo_write = (tmp as i64).min(rs2_data);
            // no err happenes here
            cpu.write(rs1_data, amo_write as u64, 8, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOMAXU_W,
        match_data: MATCH_AMOMAXU_W,
        name: "AMOMAXU_W",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2) as u32;

            let tmp = match cpu.read(rs1_data, 4, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            let amo_write = (tmp as u32).max(rs2_data);
            // no err happenes here
            cpu.write(rs1_data, amo_write as u64, 4, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp as i32 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOMAXU_D,
        match_data: MATCH_AMOMAXU_D,
        name: "AMOMAXU_D",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2);

            let tmp = match cpu.read(rs1_data, 8, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            let amo_write = tmp.max(rs2_data);
            // no err happenes here
            cpu.write(rs1_data, amo_write, 8, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOMAX_W,
        match_data: MATCH_AMOMAX_W,
        name: "AMOMAX_W",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2) as i32;

            let tmp = match cpu.read(rs1_data, 4, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            let amo_write = (tmp as i32).max(rs2_data);
            // no err happenes here
            cpu.write(rs1_data, amo_write as u64, 4, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp as i32 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOMAX_D,
        match_data: MATCH_AMOMAX_D,
        name: "AMOMAX_D",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2) as i64;

            let tmp = match cpu.read(rs1_data, 8, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            let amo_write = (tmp as i64).max(rs2_data);
            // no err happenes here
            cpu.write(rs1_data, amo_write as u64, 8, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOAND_W,
        match_data: MATCH_AMOAND_W,
        name: "AMOAND_W",
        operation: |cpu, inst, pc| {
            // Atomic Memory Operation: AND Word. R-type, RV32A and RV64A.
            // Atomically, let t be the value of the memory word at address x[rs1], then set that memory
            // word to the bitwise AND of t and x[rs2]. Set x[rd] to the sign extension of t.
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2) as u32;

            let tmp = match cpu.read(rs1_data, 4, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            let amo_write = (tmp as u32) & rs2_data;
            // no err happenes here
            cpu.write(rs1_data, amo_write as u64, 4, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp as i32 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOAND_D,
        match_data: MATCH_AMOAND_D,
        name: "AMOAND_D",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2);

            let tmp = match cpu.read(rs1_data, 8, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            let amo_write = tmp & rs2_data;
            // no err happenes here
            cpu.write(rs1_data, amo_write, 8, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOADD_W,
        match_data: MATCH_AMOADD_W,
        name: "AMOADD_W",
        operation: |cpu, inst, pc| {
            // Atomic Memory Operation: Add Word. R-type, RV32A and RV64A.
            // Atomically, let t be the value of the memory word at address x[rs1], then set that memory
            // word to t + x[rs2]. Set x[rd] to the sign extension of t.
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2) as u32;

            let tmp = match cpu.read(rs1_data, 4, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            let amo_write = (tmp as u32).wrapping_add(rs2_data);
            // no err happenes here
            cpu.write(rs1_data, amo_write as u64, 4, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp as i32 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AMOADD_D,
        match_data: MATCH_AMOADD_D,
        name: "AMOADD_D",
        operation: |cpu, inst, pc| {
            let f = parse_format_r(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let rs2_data = cpu.gpr.read(f.rs2);

            let tmp = match cpu.read(rs1_data, 8, AccessType::Amo(rs1_data)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            let amo_write = tmp.wrapping_add(rs2_data);

            // no err happenes here
            cpu.write(rs1_data, amo_write, 8, AccessType::Amo(rs1_data))
                .unwrap();
            cpu.gpr.write(f.rd, tmp);

            Ok(())
        },
    },
];
