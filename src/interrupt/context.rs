use riscv::register::{sstatus::Sstatus, scause::Scause};
use core::fmt;
use core::mem::zeroed;
#[repr(C)]
#[derive(Debug)]
pub struct Context{
    pub x: [usize; 32],    //32个通用寄存器
    pub sstatus: Sstatus,
    pub sepc: usize
}

