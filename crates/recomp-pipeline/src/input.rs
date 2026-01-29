use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Module {
    pub arch: String,
    pub functions: Vec<Function>,
}

#[derive(Debug, Deserialize)]
pub struct Function {
    pub name: String,
    pub ops: Vec<Op>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    ConstI64 { dst: String, imm: i64 },
    AddI64 { dst: String, lhs: String, rhs: String },
    Syscall { name: String, args: Vec<String> },
    Ret,
}

impl Module {
    pub fn validate_arch(&self) -> Result<(), String> {
        if self.arch != "aarch64" {
            return Err(format!("unsupported arch: {}", self.arch));
        }
        Ok(())
    }
}
