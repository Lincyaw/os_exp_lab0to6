#![allow(dead_code)]
pub mod heap;
pub mod config;
pub mod frame;
pub mod address;
pub use {
    config::*,
};

/// 初始化内存相关的子模块
///
/// - [`heap::init`]
pub fn init() {
    heap::init();
    // 允许内核读写用户态内存
    //unsafe { riscv::register::sstatus::set_sum() };

    //println!("mod memory initialized");
}
