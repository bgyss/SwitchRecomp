use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Reg {
    X(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    MovImm { dst: Reg, imm: i64 },
    Add { dst: Reg, lhs: Reg, rhs: Reg },
    Ret,
}

#[derive(Debug, Default, Clone)]
pub struct RegisterFile {
    regs: BTreeMap<Reg, i64>,
}

impl RegisterFile {
    pub fn get(&self, reg: Reg) -> i64 {
        *self.regs.get(&reg).unwrap_or(&0)
    }

    pub fn set(&mut self, reg: Reg, value: i64) {
        self.regs.insert(reg, value);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("unsupported instruction")]
    Unsupported,
}

pub fn execute_block(instructions: &[Instruction], regs: &mut RegisterFile) -> Result<(), ExecError> {
    for inst in instructions {
        match *inst {
            Instruction::MovImm { dst, imm } => regs.set(dst, imm),
            Instruction::Add { dst, lhs, rhs } => {
                let lhs_val = regs.get(lhs);
                let rhs_val = regs.get(rhs);
                regs.set(dst, lhs_val + rhs_val);
            }
            Instruction::Ret => return Ok(()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mov_and_add() {
        let mut regs = RegisterFile::default();
        let block = [
            Instruction::MovImm {
                dst: Reg::X(0),
                imm: 7,
            },
            Instruction::MovImm {
                dst: Reg::X(1),
                imm: 5,
            },
            Instruction::Add {
                dst: Reg::X(2),
                lhs: Reg::X(0),
                rhs: Reg::X(1),
            },
            Instruction::Ret,
        ];

        execute_block(&block, &mut regs).expect("exec ok");
        assert_eq!(regs.get(Reg::X(2)), 12);
    }
}
