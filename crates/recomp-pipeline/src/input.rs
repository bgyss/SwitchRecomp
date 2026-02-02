use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Module {
    pub arch: String,
    pub functions: Vec<Function>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Function {
    pub name: String,
    #[serde(default)]
    pub ops: Vec<Op>,
    #[serde(default)]
    pub blocks: Vec<Block>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Block {
    pub label: String,
    pub start: u64,
    #[serde(default)]
    pub ops: Vec<Op>,
    pub terminator: Terminator,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    ConstI64 {
        dst: String,
        imm: i64,
    },
    AddI64 {
        dst: String,
        lhs: String,
        rhs: String,
    },
    MovI64 {
        dst: String,
        src: String,
    },
    SubI64 {
        dst: String,
        lhs: String,
        rhs: String,
    },
    AndI64 {
        dst: String,
        lhs: String,
        rhs: String,
    },
    OrI64 {
        dst: String,
        lhs: String,
        rhs: String,
    },
    XorI64 {
        dst: String,
        lhs: String,
        rhs: String,
    },
    CmpI64 {
        lhs: String,
        rhs: String,
    },
    CmnI64 {
        lhs: String,
        rhs: String,
    },
    TestI64 {
        lhs: String,
        rhs: String,
    },
    LslI64 {
        dst: String,
        lhs: String,
        rhs: String,
    },
    LsrI64 {
        dst: String,
        lhs: String,
        rhs: String,
    },
    AsrI64 {
        dst: String,
        lhs: String,
        rhs: String,
    },
    PcRel {
        dst: String,
        pc: i64,
        offset: i64,
    },
    LoadI8 {
        dst: String,
        addr: String,
        #[serde(default)]
        offset: i64,
    },
    LoadI16 {
        dst: String,
        addr: String,
        #[serde(default)]
        offset: i64,
    },
    LoadI32 {
        dst: String,
        addr: String,
        #[serde(default)]
        offset: i64,
    },
    LoadI64 {
        dst: String,
        addr: String,
        #[serde(default)]
        offset: i64,
    },
    StoreI8 {
        src: String,
        addr: String,
        #[serde(default)]
        offset: i64,
    },
    StoreI16 {
        src: String,
        addr: String,
        #[serde(default)]
        offset: i64,
    },
    StoreI32 {
        src: String,
        addr: String,
        #[serde(default)]
        offset: i64,
    },
    StoreI64 {
        src: String,
        addr: String,
        #[serde(default)]
        offset: i64,
    },
    Br {
        target: String,
    },
    BrCond {
        cond: String,
        then: String,
        #[serde(rename = "else")]
        else_target: String,
    },
    Call {
        target: String,
    },
    Syscall {
        name: String,
        args: Vec<String>,
    },
    Ret,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Terminator {
    Br {
        target: String,
    },
    BrCond {
        cond: String,
        then: String,
        #[serde(rename = "else")]
        else_target: String,
    },
    Call {
        target: String,
        next: String,
    },
    BrIndirect {
        reg: String,
    },
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
