#![no_std]

use portal_pc_asm_common::types::{Arith, Bitness, Cmp, Ext, Sign};
#[macro_use]
extern crate alloc;
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Operator {
    I32(Arith),
    I64(Arith),
    I32Cmp(Cmp),
    I64Cmp(Cmp),
    I32WrapI64,
    I64Extend32(Ext),
    I32Const { value: u32 },
    I64Const { value: u64 },
    I32Load { bitness: Bitness },
    I64Load { bitness: Bitness },
    I32Store { bitness: Bitness },
    I64Store { bitness: Bitness },
}
