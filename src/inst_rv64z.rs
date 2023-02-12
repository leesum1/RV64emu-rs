use crate::{inst_base::*, traptype::TrapType};

#[allow(unused_variables)]
pub const INSTRUCTIONS_Z: [Instruction; 11] = [
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
        mask: MASK_ECALL,
        match_data: MATCH_ECALL,
        name: "ECALL",
        operation: |cpu, inst, pc| Err(TrapType::EnvironmentCallFromMMode),
    },
    Instruction {
        mask: MASK_MRET,
        match_data: MATCH_MRET,
        name: "MRET",
        operation: |cpu, inst, pc| {
            // word_t isa_intr_ret(void) {
            //     mstatus_t *mstatus_temp = (mstatus_t*)&cpu.csr[mstatus];
            //     // printf("pre isa_intr_ret mstatus:%x\n", cpu.csr[mstatus]);
            //     mstatus_temp->bit.mie = mstatus_temp->bit.mpie;
            //     mstatus_temp->bit.mpp = 0b00;
            //     mstatus_temp->bit.mpie = 1;

            //     // printf("after isa_intr_ret mstatus:%x\n", cpu.csr[mstatus]);
            //     return cpu.csr[mepc]; // 返回统一的异常处理程序地址
            //   }

            let mstatus_val = cpu.csr_regs.read(CSR_MSTATUS.into());
            let mpie = get_field(mstatus_val, MSTATUS_MPIE);
            let mstatus_val = set_field(mstatus_val, MSTATUS_MIE, mpie);
            let mstatus_val = set_field(mstatus_val, MSTATUS_MPP, 0b11);
            let mstatus_val = set_field(mstatus_val, MSTATUS_MPIE, 0b1);
            cpu.csr_regs.write(CSR_MSTATUS.into(), mstatus_val);
            let mepc = cpu.csr_regs.read(CSR_MEPC.into());
            cpu.npc = mepc;

            Ok(())
        },
    },
    Instruction {
        mask: MASK_FENCE_I,
        match_data: MATCH_FENCE_I,
        name: "FENCE_I",
        operation: |cpu, inst, pc| Ok(()),
    },
    Instruction {
        mask: MASK_FENCE,
        match_data: MATCH_FENCE,
        name: "FENCE",
        operation: |cpu, inst, pc| Ok(()),
    },
    Instruction {
        mask: MASK_CSRRC,
        match_data: MATCH_CSRRC,
        name: "CSRRC",
        operation: |cpu, inst, pc| {
            // t = CSRs[csr]; CSRs[csr] = t &∼x[rs1]; x[rd] = t
            let f = parse_format_csr(inst);
            let t = cpu.csr_regs.read(f.csr);
            let rs1_data = cpu.gpr.read(f.rs1);

            let csr_wb_data = t & !rs1_data;
            cpu.csr_regs.write(f.csr, csr_wb_data);
            cpu.gpr.write(f.rd, t);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_CSRRS,
        match_data: MATCH_CSRRS,
        name: "CSRRS",
        operation: |cpu, inst, pc| {
            // t = CSRs[csr]; CSRs[csr] = t | x[rs1]; x[rd] = t
            let f = parse_format_csr(inst);
            let t = cpu.csr_regs.read(f.csr);

            let rs1_data = cpu.gpr.read(f.rs1);

            let csr_wb_data = t | rs1_data;
            cpu.csr_regs.write(f.csr, csr_wb_data);

            cpu.gpr.write(f.rd, t);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_CSRRW,
        match_data: MATCH_CSRRW,
        name: "CSRRW",
        operation: |cpu, inst, pc| {
            // t = CSRs[csr]; CSRs[csr] = x[rs1]; x[rd] = t
            let f = parse_format_csr(inst);

            let t = cpu.csr_regs.read(f.csr);
            let rs1_data = cpu.gpr.read(f.rs1);
            let csr_wb_data = rs1_data;

            cpu.csr_regs.write(f.csr, csr_wb_data);

            cpu.gpr.write(f.rd, t);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_CSRRCI,
        match_data: MATCH_CSRRCI,
        name: "CSRRCI",
        operation: |cpu, inst, pc| {
            // t = CSRs[csr]; CSRs[csr] = t &∼zimm; x[rd] =
            let f = parse_format_csr(inst);
            let t = cpu.csr_regs.read(f.csr);
            let zimm = f.rs1;

            let csr_wb_data = t & !zimm;
            cpu.csr_regs.write(f.csr, csr_wb_data);
            cpu.gpr.write(f.rd, t);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_CSRRSI,
        match_data: MATCH_CSRRSI,
        name: "CSRRSI",
        operation: |cpu, inst, pc| {
            // t = CSRs[csr]; CSRs[csr] = t | zimm; x[rd] = t
            let f = parse_format_csr(inst);
            let t = cpu.csr_regs.read(f.csr);
            let zimm = f.rs1;

            let csr_wb_data = t | zimm;
            cpu.csr_regs.write(f.csr, csr_wb_data);
            cpu.gpr.write(f.rd, t);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_CSRRWI,
        match_data: MATCH_CSRRWI,
        name: "CSRRWI",
        operation: |cpu, inst, pc| {
            // x[rd] = CSRs[csr]; CSRs[csr] = zimm
            let f = parse_format_csr(inst);

            let t = cpu.csr_regs.read(f.csr);
            let zimm = f.rs1;
            let csr_wb_data = zimm;

            cpu.csr_regs.write(f.csr, csr_wb_data);
            cpu.gpr.write(f.rd, t);

            Ok(())
        },
    },
];
