use crate::rv64core::{inst::inst_base::*, traptype::TrapType};

#[cfg(feature = "rvc_debug_trace")]
use crate::trace::traces::TraceType;
// https://stackoverflow.com/questions/50241218/risc-v-compressed-instructions-can-compiler-be-forced-to-align-32bit-instructio
#[allow(unused_variables)]
pub const INSTRUCTIONS_C: &[Instruction] = &[
    Instruction {
        mask: MASK_C_LWSP,
        match_data: MATCH_C_LWSP,
        name: "c.lwsp",
        operation: |cpu, inst, pc| {
            let f = FormatCI::new(inst);
            let imm = f.imm_c_lwsp() as u64;
            let rd = f.rd() as u64;
            let x2 = cpu.gpr.read(2);
            let mem_addr = x2.wrapping_add(imm);

            let mem_data = match cpu.read(mem_addr, 4, AccessType::Load(mem_addr)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            cpu.gpr.write(rd, mem_data as i32 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_LDSP,
        match_data: MATCH_C_LDSP,
        name: "c.ldsp",
        operation: |cpu, inst, pc| {
            let f = FormatCI::new(inst);
            let imm = f.imm_c_ldsp() as u64;
            let rd = f.rd() as u64;
            let x2 = cpu.gpr.read(2);
            let mem_addr = x2.wrapping_add(imm);

            let mem_data = match cpu.read(mem_addr, 8, AccessType::Load(mem_addr)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            cpu.gpr.write(rd, mem_data);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_SWSP,
        match_data: MATCH_C_SWSP,
        name: "c.swsp",
        operation: |cpu, inst, pc| {
            let f = FormatCSS::new(inst);
            let imm = f.imm_c_swsp() as u64;
            let rs2 = cpu.gpr.read(f.rs2() as u64);
            let x2 = cpu.gpr.read(2);
            let mem_addr = x2.wrapping_add(imm);

            match cpu.write(
                mem_addr,
                rs2,
                4,
                AccessType::Store(mem_addr),
            ) {
                Ok(_) => Ok(()),
                Err(trap_type) => Err(trap_type),
            }
        },
    },
    Instruction {
        mask: MASK_C_SDSP,
        match_data: MATCH_C_SDSP,
        name: "c.sdsp",
        operation: |cpu, inst, pc| {
            let f = FormatCSS::new(inst);
            let imm = f.imm_c_sdsp() as u64;
            let rs2 = cpu.gpr.read(f.rs2() as u64);
            let x2 = cpu.gpr.read(2);
            let mem_addr = x2.wrapping_add(imm);

            match cpu.write(
                mem_addr,
                rs2,
                8,
                AccessType::Store(mem_addr),
            ) {
                Ok(_) => Ok(()),
                Err(trap_type) => Err(trap_type),
            }
        },
    },
    Instruction {
        mask: MASK_C_LW,
        match_data: MATCH_C_LW,
        name: "c.lw",
        operation: |cpu, inst, pc| {
            let f = FormatCL::new(inst);
            let imm = f.imm_c_lw() as u64;
            let rs1_data = cpu.gpr.read(f.rs1() as u64);
            let rd = f.rd() as u64;
            let mem_addr = rs1_data.wrapping_add(imm);

            let mem_data = match cpu.read(mem_addr, 4, AccessType::Load(mem_addr)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            cpu.gpr.write(rd, mem_data as i32 as i64 as u64);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_LD,
        match_data: MATCH_C_LD,
        name: "c.ld",
        operation: |cpu, inst, pc| {
            let f = FormatCL::new(inst);
            let imm = f.imm_c_ld() as u64;
            let rs1_data = cpu.gpr.read(f.rs1() as u64);
            let rd = f.rd() as u64;
            let mem_addr = rs1_data.wrapping_add(imm);

            let mem_data = match cpu.read(mem_addr, 8, AccessType::Load(mem_addr)) {
                Ok(data) => data,
                Err(trap_type) => return Err(trap_type),
            };
            cpu.gpr.write(rd, mem_data);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_SW,
        match_data: MATCH_C_SW,
        name: "c.sw",
        operation: |cpu, inst, pc| {
            let f = FormatCS::new(inst);
            let imm = f.imm_c_sw() as u64;
            let rs2 = cpu.gpr.read(f.rs2() as u64);
            let rs1 = cpu.gpr.read(f.rs1() as u64);
            let mem_addr = rs1.wrapping_add(imm);

            match cpu.write(
                mem_addr,
                rs2,
                4,
                AccessType::Store(mem_addr),
            ) {
                Ok(_) => Ok(()),
                Err(trap_type) => Err(trap_type),
            }
        },
    },
    Instruction {
        mask: MASK_C_SD,
        match_data: MATCH_C_SD,
        name: "c.sd",
        operation: |cpu, inst, pc| {
            let f = FormatCS::new(inst);
            let imm = f.imm_c_sd() as u64;
            let rs2 = cpu.gpr.read(f.rs2() as u64);
            let rs1 = cpu.gpr.read(f.rs1() as u64);
            let mem_addr = rs1.wrapping_add(imm);

            match cpu.write(
                mem_addr,
                rs2,
                8,
                AccessType::Store(mem_addr),
            ) {
                Ok(_) => Ok(()),
                Err(trap_type) => Err(trap_type),
            }
        },
    },
    Instruction {
        mask: MASK_C_J,
        match_data: MATCH_C_J,
        name: "c.j",
        operation: |cpu, inst, pc| {
            let f = FormatCJ::new(inst);
            let imm = f.imm_c_j() as i64;

            // todo! check align
            let next_pc = cpu.pc.wrapping_add(imm as u64);
            #[cfg(feature = "rvc_debug_trace")]
            if f.is_call() {
                if let Some(sender) = &cpu.trace_sender {
                    sender.send(TraceType::Call(pc, next_pc)).unwrap();
                };
            };
            cpu.npc = next_pc;
            Ok(())
        },
    },
    // Instruction {
    //     mask: MASK_C_JAL,
    //     match_data: MATCH_C_JAL,
    //     name: "c.jal",
    //     operation: |cpu, inst, pc| {
    //         let f = FormatCJ::new(inst);
    //         let imm = f.imm_c_jal() as i64;
    //         panic!("c.jal is rv32 only");
    //         // todo! check align
    //         let next_pc = cpu.pc.wrapping_add(imm as u64);
    //         #[cfg(feature = "rvc_debug_trace")]
    //         if f.is_call() {
    //             if let Some(sender) = &cpu.trace_sender {
    //                 sender.send(TraceType::Call(pc, next_pc)).unwrap();
    //             };
    //         };
    //         cpu.npc = next_pc;
    //         Ok(())
    //     },
    // },
    Instruction {
        mask: MASK_C_JR,
        match_data: MATCH_C_JR,
        name: "c.jr",
        operation: |cpu, inst, pc| {
            let f = FormatCR::new(inst);
            let rs1_data = cpu.gpr.read(f.rs1());

            // todo! check align
            let next_pc = rs1_data;
            #[cfg(feature = "rvc_debug_trace")]
            if f.is_call() {
                if let Some(sender) = &cpu.trace_sender {
                    sender.send(TraceType::Call(pc, next_pc)).unwrap();
                };
            };
            cpu.npc = next_pc;
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_JALR,
        match_data: MATCH_C_JALR,
        name: "c.jalr",
        operation: |cpu, inst, pc| {
            // t = pc+2; pc = x[rs1]; x[1] = t
            let f = FormatCR::new(inst);
            let rs1_data = cpu.gpr.read(f.rs1());
            let wdata = pc.wrapping_add(2);
            // todo! check align
            let next_pc = rs1_data;
            #[cfg(feature = "rvc_debug_trace")]
            if f.is_call() {
                if let Some(sender) = &cpu.trace_sender {
                    sender.send(TraceType::Call(pc, next_pc)).unwrap();
                };
            };
            cpu.npc = next_pc;
            cpu.gpr.write(1, wdata);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_BEQZ,
        match_data: MATCH_C_BEQZ,
        name: "c.beqz",
        operation: |cpu, inst, pc| {
            // if (rs1 == rs2) pc += sext(offset)
            let f = FormatCB::new(inst);
            let rs1 = cpu.gpr.read(f.rs1() as u64);
            let imm = f.imm_c_beqz() as i64 as u64;

            if rs1 == 0 {
                let next_pc = pc.wrapping_add(imm);
                if !check_aligned(next_pc, 2) {
                    // todo! not clear
                    return Err(TrapType::InstructionAddressMisaligned(next_pc));
                }
                cpu.npc = next_pc;
            }
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_BNEZ,
        match_data: MATCH_C_BNEZ,
        name: "c.bnez",
        operation: |cpu, inst, pc| {
            let f = FormatCB::new(inst);
            let rs1 = cpu.gpr.read(f.rs1() as u64);
            let imm = f.imm_c_bnez() as i64 as u64;

            if rs1 != 0 {
                let next_pc = pc.wrapping_add(imm);
                if !check_aligned(next_pc, 2) {
                    // todo! not clear
                    return Err(TrapType::InstructionAddressMisaligned(next_pc));
                }
                cpu.npc = next_pc;
            }
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_LI,
        match_data: MATCH_C_LI,
        name: "c.li",
        operation: |cpu, inst, pc| {
            let f = FormatCI::new(inst);
            let rd = f.rd();
            let imm = f.imm_c_li() as i64 as u64;

            cpu.gpr.write(rd as u64, imm);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_LUI,
        match_data: MATCH_C_LUI,
        name: "c.lui",
        operation: |cpu, inst, pc| {
            let f = FormatCI::new(inst);
            let rd = f.rd();
            let imm = f.imm_c_lui() as i64 as u64;

            cpu.gpr.write(rd as u64, imm);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_ADDI,
        match_data: MATCH_C_ADDI,
        name: "c.addi",
        operation: |cpu, inst, pc| {
            let f = FormatCI::new(inst);
            let imm = f.imm_c_addi() as i64 as u64;
            let rd = f.rd() as u64;
            let rd_data = cpu.gpr.read(rd);

            let wb = rd_data.wrapping_add(imm);
            cpu.gpr.write(rd, wb);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_ADDIW,
        match_data: MATCH_C_ADDIW,
        name: "c.addiw",
        operation: |cpu, inst, pc| {
            let f = FormatCI::new(inst);
            let imm = f.imm_c_addiw() as i64 as u64;
            let rd = f.rd() as u64;
            let rd_data = cpu.gpr.read(rd);

            let wb = rd_data.wrapping_add(imm) as i32;
            cpu.gpr.write(rd, wb as i64 as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_ADDI16SP,
        match_data: MATCH_C_ADDI16SP,
        name: "c.addi16sp",
        operation: |cpu, inst, pc| {
            let f = FormatCI::new(inst);
            let imm = f.imm_c_addi16sp() as i64;
            let rd = 2;
            let rd_data = cpu.gpr.read(rd) as i64;
            
            let wb = rd_data.wrapping_add(imm);

            cpu.gpr.write(rd, wb as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_ADDI4SPN,
        match_data: MATCH_C_ADDI4SPN,
        name: "c.addi4spn",
        operation: |cpu, inst, pc| {
            let f = FormatCIW::new(inst);
            let imm = f.imm_c_addi4spn() as u64 as i64;
            let x2_data = cpu.gpr.read(2) as i64;

            let rd: u64 = f.rd() as u64;

            let wb = x2_data.wrapping_add(imm);
            cpu.gpr.write(rd, wb as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_SLLI,
        match_data: MATCH_C_SLLI,
        name: "c.slli",
        operation: |cpu, inst, pc| {
            let f = FormatCI::new(inst);
            let shamt = f.imm_c_slli() as u64;
            let rd: u64 = f.rd() as u64;
            let rd_data = cpu.gpr.read(rd);

            let wb = rd_data << shamt;
            cpu.gpr.write(rd, wb);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_SRLI,
        match_data: MATCH_C_SRLI,
        name: "c.srli",
        operation: |cpu, inst, pc| {
            let f = FormatCB::new(inst);
            let shamt = f.imm_c_srli() as u64;
            let rd: u64 = f.rd() as u64;
            let rd_data = cpu.gpr.read(rd);

            let wb = rd_data >> shamt;
            cpu.gpr.write(rd, wb);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_SRAI,
        match_data: MATCH_C_SRAI,
        name: "c.srai",
        operation: |cpu, inst, pc| {
            let f = FormatCB::new(inst);
            let shamt = f.imm_c_srai() as u64;
            let rd = f.rd() as u64;
            let rd_data = cpu.gpr.read(rd) as i64;

            let wb = rd_data >> shamt;
            cpu.gpr.write(rd, wb as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_ANDI,
        match_data: MATCH_C_ANDI,
        name: "c.andi",
        operation: |cpu, inst, pc| {
            let f = FormatCB::new(inst);
            let imm = f.imm_c_andi() as i64;
            let rd: u64 = f.rd() as u64;
            let rd_data = cpu.gpr.read(rd) as i64;

            let wb = rd_data & imm;
            cpu.gpr.write(rd, wb as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_MV,
        match_data: MATCH_C_MV,
        name: "c.mv",
        operation: |cpu, inst, pc| {
            let f = FormatCR::new(inst);

            let rd = f.rd();
            let rs2 = f.rs2();

            let rs2_data = cpu.gpr.read(rs2);

            let wb = rs2_data;
            cpu.gpr.write(rd, wb);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_ADD,
        match_data: MATCH_C_ADD,
        name: "c.add",
        operation: |cpu, inst, pc| {
            let f = FormatCR::new(inst);

            let rd: u64 = f.rd();
            let rs2 = f.rs2();

            let rs2_data = cpu.gpr.read(rs2) as i64;
            let rd_data = cpu.gpr.read(rd) as i64;

            let wb = rs2_data.wrapping_add(rd_data);
            cpu.gpr.write(rd, wb as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_AND,
        match_data: MATCH_C_AND,
        name: "c.and",
        operation: |cpu, inst, pc| {
            let f = FormatCA::new(inst);

            let rd: u64 = f.rd() as u64;
            let rs2 = f.rs2() as u64;

            let rs2_data = cpu.gpr.read(rs2);
            let rd_data = cpu.gpr.read(rd);

            let wb = rs2_data & rd_data;
            cpu.gpr.write(rd, wb);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_OR,
        match_data: MATCH_C_OR,
        name: "c.or",
        operation: |cpu, inst, pc| {
            let f = FormatCA::new(inst);

            let rd: u64 = f.rd() as u64;
            let rs2 = f.rs2() as u64;

            let rs2_data = cpu.gpr.read(rs2);
            let rd_data = cpu.gpr.read(rd);

            let wb = rs2_data | rd_data;
            cpu.gpr.write(rd, wb);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_XOR,
        match_data: MATCH_C_XOR,
        name: "c.xor",
        operation: |cpu, inst, pc| {
            let f = FormatCA::new(inst);

            let rd: u64 = f.rd() as u64;
            let rs2 = f.rs2() as u64;

            let rs2_data = cpu.gpr.read(rs2);
            let rd_data = cpu.gpr.read(rd);

            let wb = rs2_data ^ rd_data;
            cpu.gpr.write(rd, wb);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_SUB,
        match_data: MATCH_C_SUB,
        name: "c.sub",
        operation: |cpu, inst, pc| {
            let f = FormatCA::new(inst);

            let rd: u64 = f.rd() as u64;
            let rs2 = f.rs2() as u64;

            let rs2_data = cpu.gpr.read(rs2) as i64;
            let rd_data = cpu.gpr.read(rd) as i64;

            let wb = rd_data.wrapping_sub(rs2_data);
            cpu.gpr.write(rd, wb as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_ADDW,
        match_data: MATCH_C_ADDW,
        name: "c.addw",
        operation: |cpu, inst, pc| {
            let f = FormatCA::new(inst);

            let rd: u64 = f.rd() as u64;
            let rs2 = f.rs2() as u64;

            let rs2_data = cpu.gpr.read(rs2) as i64;
            let rd_data = cpu.gpr.read(rd) as i64;

            let wb = rs2_data.wrapping_add(rd_data) as i32;
            cpu.gpr.write(rd, wb as i64 as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_SUBW,
        match_data: MATCH_C_SUBW,
        name: "c.subw",
        operation: |cpu, inst, pc| {
            let f = FormatCA::new(inst);

            let rd: u64 = f.rd() as u64;
            let rs2 = f.rs2() as u64;

            let rs2_data = cpu.gpr.read(rs2) as i64;
            let rd_data = cpu.gpr.read(rd) as i64;

            let wb = rd_data.wrapping_sub(rs2_data) as i32;
            cpu.gpr.write(rd, wb as i64 as u64);
            Ok(())
        },
    },
    Instruction {
        mask: MASK_C_NOP,
        match_data: MATCH_C_NOP,
        name: "c.nop",
        operation: |cpu, inst, pc| Ok(()),
    },
    Instruction {
        mask: MASK_C_EBREAK,
        match_data: MATCH_C_EBREAK,
        name: "c.ebreak",
        operation: |cpu, inst, pc| {
            #[cfg(feature = "support_am")]
            cpu.halt();
            Err(TrapType::Breakpoint(pc))
        },
    },
];
