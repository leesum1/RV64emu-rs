use crate::{inst::inst_base::*, traptype::TrapType};

#[allow(unused_variables)]
pub const INSTRUCTIONS_I: [Instruction; 49] = [
    Instruction {
        mask: MASK_LUI,
        match_data: MATCH_LUI,
        name: "lui",
        operation: |cpu, inst, pc| {
            let f = parse_format_u(inst);
            cpu.gpr.write(f.rd, f.imm);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_AUIPC,
        match_data: MATCH_AUIPC,
        name: "auipc",
        operation: |cpu, inst, pc| {
            let f = parse_format_u(inst);
            let wdata = pc.wrapping_add(f.imm);
            cpu.gpr.write(f.rd, wdata);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_JAL,
        match_data: MATCH_JAL,
        name: "jal",
        operation: |cpu, inst, pc| {
            let f = parse_format_j(inst);
            let wdata = pc.wrapping_add(4);

            let next_pc = pc.wrapping_add(f.imm);
            if !check_aligned(next_pc, 4) {
                // todo! not clear
                return Err(TrapType::InstructionAddressMisaligned(next_pc));
            };

            #[cfg(feature = "rv_debug_trace")]
            if f.is_call() {
                if let Some(sender) = &cpu.trace_sender {
                    sender.send(TraceType::Call(pc, next_pc)).unwrap();
                };
            };
            cpu.npc = next_pc;
            cpu.gpr.write(f.rd, wdata);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_JALR,
        match_data: MATCH_JALR,
        name: "jalr",
        operation: |cpu, inst, pc| {
            // t =pc+4; pc=(x[rs1]+sext(offset))&∼1; x[rd]=t
            let f = parse_format_i(inst);
            let rs1_data = cpu.gpr.read(f.rs1);
            let wdata = pc.wrapping_add(4);

            let next_pc = (rs1_data.wrapping_add(f.imm as u64)) & !1_u64;

            if !check_aligned(next_pc, 4) {
                // todo! not clear
                return Err(TrapType::InstructionAddressMisaligned(next_pc));
            };
            
            #[cfg(feature = "rv_debug_trace")]
            if let Some(val) = f.get_jalr_type() {
                if let Some(sender) = &cpu.trace_sender {
                    match val {
                        true => sender.send(TraceType::Return(pc, next_pc)).unwrap(),
                        false => sender.send(TraceType::Call(pc, next_pc)).unwrap(),
                    }
                };
            };

            cpu.npc = next_pc;
            cpu.gpr.write(f.rd, wdata);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_BEQ,
        match_data: MATCH_BEQ,
        name: "BEQ",
        operation: |cpu, inst, pc| {
            // if (rs1 == rs2) pc += sext(offset)
            let f = parse_format_b(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let rs2 = cpu.gpr.read(f.rs2);

            if rs1 == rs2 {
                let next_pc = pc.wrapping_add(f.imm);
                if !check_aligned(next_pc, 4) {
                    // todo! not clear
                    return Err(TrapType::InstructionAddressMisaligned(next_pc));
                }
                cpu.npc = next_pc;
            }
            Ok(())
        },
    },
    Instruction {
        mask: MASK_BNE,
        match_data: MATCH_BNE,
        name: "BNE",
        operation: |cpu, inst, pc| {
            // if (rs1 != rs2) pc += sext(offset)
            let f = parse_format_b(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let rs2 = cpu.gpr.read(f.rs2);

            if rs1 != rs2 {
                let next_pc = pc.wrapping_add(f.imm);
                if !check_aligned(next_pc, 4) {
                    // todo! not clear
                    return Err(TrapType::InstructionAddressMisaligned(next_pc));
                }
                cpu.npc = next_pc;
            }
            Ok(())
        },
    },
    Instruction {
        mask: MASK_BLT,
        match_data: MATCH_BLT,
        name: "BLT",
        operation: |cpu, inst, pc| {
            // if (rs1 <(sign) rs2) pc += sext(offset)
            let f = parse_format_b(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as i64;

            if rs1 < rs2 {
                let next_pc = pc.wrapping_add(f.imm);
                if !check_aligned(next_pc, 4) {
                    return Err(TrapType::InstructionAddressMisaligned(next_pc));
                }
                cpu.npc = next_pc;
            }
            Ok(())
        },
    },
    Instruction {
        mask: MASK_BGE,
        match_data: MATCH_BGE,
        name: "BGE",
        operation: |cpu, inst, pc| {
            // if (rs1 ≥(sign) rs2) pc += sext(offset)
            let f = parse_format_b(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as i64;

            if rs1 >= rs2 {
                let next_pc = pc.wrapping_add(f.imm);
                if !check_aligned(next_pc, 4) {
                    return Err(TrapType::InstructionAddressMisaligned(next_pc));
                }
                cpu.npc = next_pc;
            }
            Ok(())
        },
    },
    Instruction {
        mask: MASK_BLTU,
        match_data: MATCH_BLTU,
        name: "BLTU",
        operation: |cpu, inst, pc| {
            // if (rs1 <u rs2) pc += sext(offset)
            let f = parse_format_b(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let rs2 = cpu.gpr.read(f.rs2);

            if rs1 < rs2 {
                let next_pc = pc.wrapping_add(f.imm);
                if !check_aligned(next_pc, 4) {
                    return Err(TrapType::InstructionAddressMisaligned(next_pc));
                }
                cpu.npc = next_pc;
            }
            Ok(())
        },
    },
    Instruction {
        mask: MASK_BGEU,
        match_data: MATCH_BGEU,
        name: "BGEU",
        operation: |cpu, inst, pc| {
            // if (rs1 ≥u rs2) pc += sext(offset)
            let f = parse_format_b(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let rs2 = cpu.gpr.read(f.rs2);
            if rs1 >= rs2 {
                let next_pc = pc.wrapping_add(f.imm);
                if !check_aligned(next_pc, 4) {
                    return Err(TrapType::InstructionAddressMisaligned(next_pc));
                }
                cpu.npc = next_pc;
            }

            Ok(())
        },
    },
    // load store
    Instruction {
        mask: MASK_LB,
        match_data: MATCH_LB,
        name: "LB",
        operation: |cpu, inst, pc| {
            // x[rd] = sext(M[x[rs1] + sext(offset)][7:0])
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let mem_addr = rs1.wrapping_add(f.imm);

            let mem_data = match cpu.read(mem_addr as u64, 1, AccessType::Load(mem_addr as u64)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };

            cpu.gpr.write(f.rd, mem_data as i8 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_LH,
        match_data: MATCH_LH,
        name: "LH",
        operation: |cpu, inst, pc| {
            // x[rd] = sext(M[x[rs1] + sext(offset)][15:0])
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let mem_addr = rs1.wrapping_add(f.imm);

            let mem_data = match cpu.read(mem_addr as u64, 2, AccessType::Load(mem_addr as u64)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            cpu.gpr.write(f.rd, mem_data as i16 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_LW,
        match_data: MATCH_LW,
        name: "LW ",
        operation: |cpu, inst, pc| {
            // x[rd] = sext(M[x[rs1] + sext(offset)][31:0])
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let mem_addr = rs1.wrapping_add(f.imm);

            let mem_data = match cpu.read(mem_addr as u64, 4, AccessType::Load(mem_addr as u64)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            cpu.gpr.write(f.rd, mem_data as i32 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_LBU,
        match_data: MATCH_LBU,
        name: "LBU ",
        operation: |cpu, inst, pc| {
            // x[rd] = M[x[rs1] + sext(offset)][7:0]
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let mem_addr = rs1.wrapping_add(f.imm);

            let mem_data = match cpu.read(mem_addr as u64, 1, AccessType::Load(mem_addr as u64)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            cpu.gpr.write(f.rd, mem_data as u8 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_LHU,
        match_data: MATCH_LHU,
        name: "LHU",
        operation: |cpu, inst, pc| {
            // x[rd] = M[x[rs1] + sext(offset)][15:0]
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let mem_addr = rs1.wrapping_add(f.imm);

            let mem_data = match cpu.read(mem_addr as u64, 2, AccessType::Load(mem_addr as u64)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            cpu.gpr.write(f.rd, mem_data as u16 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SB,
        match_data: MATCH_SB,
        name: "SB",
        operation: |cpu, inst, pc| {
            // M[x[rs1] + sext(offset)] = x[rs2][7:0]
            let f = parse_format_s(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as u8;
            let mem_addr = rs1.wrapping_add(f.imm);

            // sb never misaligned
            // if cpu.write(mem_addr as u64, rs2 as u64, 1).is_err() {
            //     return Err(TrapType::StoreAddressMisaligned);
            // }

            match cpu.write(
                mem_addr as u64,
                rs2 as u64,
                1,
                AccessType::Store(mem_addr as u64),
            ) {
                Ok(_) => Ok(()),
                Err(trap_type) => Err(trap_type),
            }
        },
    },
    Instruction {
        mask: MASK_SH,
        match_data: MATCH_SH,
        name: "SH",
        operation: |cpu, inst, pc| {
            // M[x[rs1] + sext(offset)] = x[rs2][15:0]
            let f = parse_format_s(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as u16;
            let mem_addr = rs1.wrapping_add(f.imm);
            match cpu.write(
                mem_addr as u64,
                rs2 as u64,
                2,
                AccessType::Store(mem_addr as u64),
            ) {
                Ok(_) => Ok(()),
                Err(trap_type) => Err(trap_type),
            }
        },
    },
    Instruction {
        mask: MASK_SW,
        match_data: MATCH_SW,
        name: "SW",
        operation: |cpu, inst, pc| {
            // M[x[rs1] + sext(offset)] = x[rs2][31:0]
            let f = parse_format_s(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as u32;
            let mem_addr = rs1.wrapping_add(f.imm);
            match cpu.write(
                mem_addr as u64,
                rs2 as u64,
                4,
                AccessType::Store(mem_addr as u64),
            ) {
                Ok(_) => Ok(()),
                Err(trap_type) => Err(trap_type),
            }
        },
    },
    Instruction {
        mask: MASK_ADDI,
        match_data: MATCH_ADDI,
        name: "ADDI",
        operation: |cpu, inst, pc| {
            // x[rd] = x[rs1] + sext(immediate)
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let wb_data = rs1.wrapping_add(f.imm);
            cpu.gpr.write(f.rd, wb_data as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SLTI,
        match_data: MATCH_SLTI,
        name: "SLTI",
        operation: |cpu, inst, pc| {
            // x[rd] = x[rs1] <s sext(immediate)
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;

            let wb_data = rs1 < f.imm;

            cpu.gpr.write(f.rd, wb_data as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SLTIU,
        match_data: MATCH_SLTIU,
        name: "SLTIU",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] <u sext(immediate)
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1);

            let wb_data = rs1 < f.imm as u64;

            cpu.gpr.write(f.rd, wb_data as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_XORI,
        match_data: MATCH_XORI,
        name: "XORI",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] ˆ sext(immediate)
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1);

            let wb_data = rs1 ^ f.imm as u64;
            cpu.gpr.write(f.rd, wb_data);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_ORI,
        match_data: MATCH_ORI,
        name: "ORI",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] | sext(immediate)
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1);

            let wb_data = rs1 | f.imm as u64;
            cpu.gpr.write(f.rd, wb_data);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_ANDI,
        match_data: MATCH_ANDI,
        name: "ANDI",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] & sext(immediate)
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1);

            let wb_data = rs1 & f.imm as u64;
            cpu.gpr.write(f.rd, wb_data);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SLLI,
        match_data: MATCH_SLLI,
        name: "SLLI",
        operation: |cpu, inst, pc| {
            //   x[rd] = x[rs1] << shamt
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let shamt = (f.imm & 0x3f) as u64;

            let wb_data = rs1 << shamt;
            cpu.gpr.write(f.rd, wb_data);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SRLI,
        match_data: MATCH_SRLI,
        name: "SRLI",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] >>u shamt
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let shamt = (f.imm & 0x3f) as u64;

            let wb_data = rs1 >> shamt;
            cpu.gpr.write(f.rd, wb_data);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SRAI,
        match_data: MATCH_SRAI,
        name: "SRAI",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] >>s shamt
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let shamt = f.imm & 0x3f;

            let wb_data = rs1 >> shamt;
            cpu.gpr.write(f.rd, wb_data as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_ADD,
        match_data: MATCH_ADD,
        name: "ADD",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] + x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as i64;

            let wb_data = rs1.wrapping_add(rs2);
            cpu.gpr.write(f.rd, wb_data as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SUB,
        match_data: MATCH_SUB,
        name: "SUB",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] - x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as i64;

            let wb_data = rs1.wrapping_sub(rs2);
            cpu.gpr.write(f.rd, wb_data as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SLL,
        match_data: MATCH_SLL,
        name: "SLL",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] << x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as i64;

            // let wb_data = rs1 << rs2;
            let wb_data = rs1.wrapping_shl(rs2 as u32);
            cpu.gpr.write(f.rd, wb_data as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SLT,
        match_data: MATCH_SLT,
        name: "SLT",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] <s x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as i64;

            let wb_data = rs1 < rs2;
            cpu.gpr.write(f.rd, wb_data as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SLTU,
        match_data: MATCH_SLTU,
        name: "SLTU",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] <u x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let rs2 = cpu.gpr.read(f.rs2);

            let wb_data = rs1 < rs2;
            cpu.gpr.write(f.rd, wb_data as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_XOR,
        match_data: MATCH_XOR,
        name: "XOR",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] ˆ x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let rs2 = cpu.gpr.read(f.rs2);

            let wb_data = rs1 ^ rs2;
            cpu.gpr.write(f.rd, wb_data);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SRL,
        match_data: MATCH_SRL,
        name: "SRL",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] >>u x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let rs2 = cpu.gpr.read(f.rs2);

            let wb_data = rs1.wrapping_shr(rs2 as u32);
            cpu.gpr.write(f.rd, wb_data);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SRA,
        match_data: MATCH_SRA,
        name: "SRA",
        operation: |cpu, inst, pc| {
            //  x[rd] = x[rs1] >>s x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as i64;

            let wb_data = rs1.wrapping_shr(rs2 as u32);
            cpu.gpr.write(f.rd, wb_data as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_OR,
        match_data: MATCH_OR,
        name: "OR",
        operation: |cpu, inst, pc| {
            //   x[rd] = x[rs1] | x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let rs2 = cpu.gpr.read(f.rs2);

            let wb_data = rs1 | rs2;
            cpu.gpr.write(f.rd, wb_data);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_AND,
        match_data: MATCH_AND,
        name: "AND",
        operation: |cpu, inst, pc| {
            //   x[rd] = x[rs1] & x[rs2]
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let rs2 = cpu.gpr.read(f.rs2);

            let wb_data = rs1 & rs2;
            cpu.gpr.write(f.rd, wb_data);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_LWU,
        match_data: MATCH_LWU,
        name: "LWU",
        operation: |cpu, inst, pc| {
            // x[rd] = M[x[rs1] + sext(offset)][31:0]
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let mem_addr = rs1.wrapping_add(f.imm);

            let mem_data = match cpu.read(mem_addr as u64, 4, AccessType::Load(mem_addr as u64)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            cpu.gpr.write(f.rd, mem_data as u32 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_LD,
        match_data: MATCH_LD,
        name: "LD",
        operation: |cpu, inst, pc| {
            // x[rd] = M[x[rs1] + sext(offset)][63:0]
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let mem_addr = rs1.wrapping_add(f.imm);

            let mem_data = match cpu.read(mem_addr as u64, 8, AccessType::Load(mem_addr as u64)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            cpu.gpr.write(f.rd, mem_data);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SD,
        match_data: MATCH_SD,
        name: "SD",
        operation: |cpu, inst, pc| {
            //  M[x[rs1] + sext(offset)] = x[rs2][63:0]
            let f = parse_format_s(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2);
            let mem_addr = rs1.wrapping_add(f.imm);
            match cpu.write(mem_addr as u64, rs2, 8, AccessType::Store(mem_addr as u64)) {
                Ok(_) => Ok(()),
                Err(trap_type) => Err(trap_type),
            }
        },
    },
    Instruction {
        mask: MASK_ADDIW,
        match_data: MATCH_ADDIW,
        name: "ADDIW",
        operation: |cpu, inst, pc| {
            //  x[rd] = sext((x[rs1] + sext(immediate))[31:0])
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let wb_data = rs1.wrapping_add(f.imm) as i32;
            cpu.gpr.write(f.rd, wb_data as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SLLIW,
        match_data: MATCH_SLLIW,
        name: "SLLIW",
        operation: |cpu, inst, pc| {
            //   x[rd] = sext((x[rs1] << shamt)[31:0])
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1);
            let shamt = (f.imm & 0x1f) as u64;

            let wb_data = rs1 << f.imm;
            cpu.gpr.write(f.rd, wb_data as i32 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SRLIW,
        match_data: MATCH_SRLIW,
        name: "SRLIW",
        operation: |cpu, inst, pc| {
            //  x[rd] = sext(x[rs1][31:0] >>u shamt)
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1) as u32;
            let shamt = (f.imm & 0x1f) as u32;

            let wb_data = rs1 >> f.imm;
            cpu.gpr.write(f.rd, wb_data as i32 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SRAIW,
        match_data: MATCH_SRAIW,
        name: "SRAIW",
        operation: |cpu, inst, pc| {
            //  x[rd] = sext(x[rs1][31:0] >>s shamt)
            let f = parse_format_i(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i32;
            let shamt = (f.imm & 0x1f) as i32;

            let wb_data = rs1.wrapping_shr(shamt as u32);
            cpu.gpr.write(f.rd, wb_data as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_ADDW,
        match_data: MATCH_ADDW,
        name: "ADDW",
        operation: |cpu, inst, pc| {
            //  x[rd] = sext((x[rs1] + x[rs2])[31:0])
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as i64;

            let wb_data = rs1.wrapping_add(rs2) as i32;
            cpu.gpr.write(f.rd, wb_data as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SUBW,
        match_data: MATCH_SUBW,
        name: "SUBW",
        operation: |cpu, inst, pc| {
            //  x[rd] = sext((x[rs1] - x[rs2])[31:0])
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) as i64;

            let wb_data = rs1.wrapping_sub(rs2) as i32;
            cpu.gpr.write(f.rd, wb_data as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SLLW,
        match_data: MATCH_SLLW,
        name: "SLLW",
        operation: |cpu, inst, pc| {
            //  x[rd] = sext((x[rs1] << x[rs2][4:0])[31:0])
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i64;
            let rs2 = cpu.gpr.read(f.rs2) & 0x1f;

            let wb_data = (rs1 << rs2) as i32;
            cpu.gpr.write(f.rd, wb_data as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SRLW,
        match_data: MATCH_SRLW,
        name: "SRLW",
        operation: |cpu, inst, pc| {
            //  x[rd] = sext(x[rs1][31:0] >>u x[rs2][4:0])
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as u32;
            let rs2 = cpu.gpr.read(f.rs2) & 0x1f;

            let wb_data = (rs1 >> rs2) as i32;
            cpu.gpr.write(f.rd, wb_data as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SRAW,
        match_data: MATCH_SRAW,
        name: "SRAW",
        operation: |cpu, inst, pc| {
            //  x[rd] = sext(x[rs1][31:0] >>s x[rs2][4:0])
            let f = parse_format_r(inst);
            let rs1 = cpu.gpr.read(f.rs1) as i32;
            let rs2 = cpu.gpr.read(f.rs2) & 0x1f;

            let wb_data = rs1 >> rs2;
            cpu.gpr.write(f.rd, wb_data as i64 as u64);

            Ok(())
        },
    },
];
#[cfg(test)]
mod test_rv64i {
    use log::warn;


    #[test]
    fn tset1() {
        let x = 1 < 2;
        warn!("x:{}", x as u64);
    }
}
