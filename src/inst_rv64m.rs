use crate::inst_base::*;
// ┌───────────────────────┬──────────┬─────────┬──────────┬───────────┬──────────┬──────────┐
// │ Condition             │ Dividend │  Dvisor │  DIVU[W] │   REMU[W] │   DIV[W ]│  REM[W]  │
// ├───────────────────────┼──────────┼─────────┼──────────┼───────────┼──────────┼──────────┤
// │ Division by zero      │    X     │    0    │   2^L-1  │     X     │    -1    │    X     │
// ├───────────────────────┼──────────┼─────────┼──────────┼───────────┼──────────┼──────────┤
// │ Overflow (signed only)│ -2^(L-1) │   -1    │     -    │      -    │  -2^(L-1)│    0     │
// └───────────────────────┴──────────┴─────────┴──────────┴───────────┴──────────┴──────────┘
//  Table 7.1: Semantics for division by zero and division overflow. L is the width of the operation in
//  bits: XLEN for DIV[U] and REM[U], or 32 for DIV[U]W and REM[U]W.

pub const INSTRUCTIONS_M: [Instruction; 13] = [
    Instruction {
        mask: MASK_MUL,
        match_data: MATCH_MUL,
        name: "MUL",
        operation: |cpu, inst, pc| {
            // x[rd] = x[rs1] × x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let rs2 = cpu.gpr.read(f.rs2);

            let wb_data = rs1.wrapping_mul(rs2);
            cpu.gpr.write(f.rd, wb_data);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_MULH,
        match_data: MATCH_MULH,
        name: "MULH",
        operation: |cpu, inst, pc| {
            // x[rd] = (x[rs1] s ×s x[rs2]) >>s XLEN
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64 as i128;
            let rs2 = cpu.gpr.read(f.rs2) as i64 as i128;

            let (mul_data, _) = rs1.overflowing_mul(rs2);
            let wb_data = (mul_data >> 64) as i64;

            cpu.gpr.write(f.rd, wb_data as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_MULHSU,
        match_data: MATCH_MULHSU,
        name: "MULHSU",
        operation: |cpu, inst, pc| {
            // x[rd] = (x[rs1] s ×u x[rs2]) >>s XLEN
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64 as i128;
            let rs2 = cpu.gpr.read(f.rs2) as u128 as i128;

            let (mul_data, _) = rs1.overflowing_mul(rs2);
            let wb_data = (mul_data >> 64) as i64;

            cpu.gpr.write(f.rd, wb_data as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_MULHU,
        match_data: MATCH_MULHU,
        name: "MULHSU",
        operation: |cpu, inst, pc| {
            //  x[rd] = (x[rs1] u×u x[rs2]) >>u XLEN
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as u128;
            let rs2 = cpu.gpr.read(f.rs2) as u128;

            let mul_data = rs1.wrapping_mul(rs2);
            let wb_data = mul_data >> 64;

            cpu.gpr.write(f.rd, wb_data as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_DIV,
        match_data: MATCH_DIV,
        name: "DIV",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] ÷s x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as i64;

            let mut wb_data;
            if rs2 == 0 {
                wb_data = -1;
            } else if rs1 == i64::MIN && rs2 == -1 {
                wb_data = i64::MIN;
            } else {
                wb_data = rs1.wrapping_div(rs2);
            }
            cpu.gpr.write(f.rd, wb_data as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_DIVU,
        match_data: MATCH_DIVU,
        name: "DIVU",
        operation: |cpu, inst, pc| {
            //   x[rd] = x[rs1] ÷u x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as u64;
            let rs2 = cpu.gpr.read(f.rs2) as u64;

            let mut wb_data: i64;
            if rs2 == 0 {
                wb_data = -1;
            } else {
                wb_data = (rs1.wrapping_div(rs2)) as i64;
            }
            cpu.gpr.write(f.rd, wb_data as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_REM,
        match_data: MATCH_REM,
        name: "REM",
        operation: |cpu, inst, pc| {
            //   x[rd] = x[rs1] %s x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as i64;

            let mut wb_data: i64;
            if rs2 == 0 {
                wb_data = rs1;
            } else if rs1 == i64::MIN && rs2 == -1 {
                wb_data = 0;
            } else {
                wb_data = rs1.wrapping_rem(rs2);
            }
            cpu.gpr.write(f.rd, wb_data as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_REMU,
        match_data: MATCH_REMU,
        name: "REMU",
        operation: |cpu, inst, pc| {
            //   x[rd] = x[rs1] % u x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as u64;
            let rs2 = cpu.gpr.read(f.rs2) as u64;

            let mut wb_data: i64;
            if rs2 == 0 {
                wb_data = rs1 as i64;
            } else {
                wb_data = (rs1.wrapping_rem(rs2)) as i64;
            }
            cpu.gpr.write(f.rd, wb_data as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_MULW,
        match_data: MATCH_MULW,
        name: "MULW",
        operation: |cpu, inst, pc| {
            // x[rd] = sext((x[rs1] × x[rs2])[31:0])
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let rs2 = cpu.gpr.read(f.rs2);

            let wb_data = (rs1.wrapping_mul(rs2)) as u32 as i32 as i64;

            cpu.gpr.write(f.rd, wb_data as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_DIVW,
        match_data: MATCH_DIVW,
        name: "DIVW",
        operation: |cpu, inst, pc| {
            //  x[rd] = sext(x[rs1][31:0] ÷s x[rs2][31:0])
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i32;
            let rs2 = cpu.gpr.read(f.rs2) as i32;

            let mut wb_data;
            if rs2 == 0 {
                wb_data = -1;
            } else if rs1 == i32::MIN && rs2 == -1 {
                wb_data = i32::MIN;
            } else {
                wb_data = rs1.wrapping_div(rs2);
            }
            cpu.gpr.write(f.rd, wb_data as i64 as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_DIVUW,
        match_data: MATCH_DIVUW,
        name: "DIVUW",
        operation: |cpu, inst, pc| {
            //   x[rd] = sext(x[rs1][31:0] ÷u x[rs2][31:0])
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as u32;
            let rs2 = cpu.gpr.read(f.rs2) as u32;

            let mut wb_data: i32;
            if rs2 == 0 {
                wb_data = -1;
            } else {
                wb_data = (rs1.wrapping_div(rs2)) as i32;
            }
            cpu.gpr.write(f.rd, wb_data as i64 as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_REMW,
        match_data: MATCH_REMW,
        name: "REMW",
        operation: |cpu, inst, pc| {
            //   x[rd] = sext(x[rs1][31:0] %s x[rs2][31:0])
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i32;
            let rs2 = cpu.gpr.read(f.rs2) as i32;

            let mut wb_data: i32;
            if rs2 == 0 {
                wb_data = rs1;
            } else if rs1 == i32::MIN && rs2 == -1 {
                wb_data = 0;
            } else {
                wb_data = rs1.wrapping_rem(rs2);
            }
            cpu.gpr.write(f.rd, wb_data as i64 as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_REMUW,
        match_data: MATCH_REMUW,
        name: "REMUW",
        operation: |cpu, inst, pc| {
            //   x[rd] = sext(x[rs1][31:0] %u x[rs2][31:0])
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as u32;
            let rs2 = cpu.gpr.read(f.rs2) as u32;

            let mut wb_data: i32;
            if rs2 == 0 {
                wb_data = rs1 as i32;
            } else {
                wb_data = (rs1.wrapping_rem(rs2)) as i32;
            }
            cpu.gpr.write(f.rd, wb_data as i64 as u64);
            Ok(())
        },
    },
];
