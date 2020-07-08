//! # 全局属性
//!   禁用标准库
#![no_std]
//! - `#![no_main]`  
//!   不使用 `main` 函数等全部 Rust-level 入口点来作为程序入口
#![no_main]
#![feature(llvm_asm)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]
#[macro_use]
mod console;
mod panic;
mod sbi;
mod interrupt;
mod memory;
extern crate alloc;
global_asm!(include_str!("asm/entry.asm"));
#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    interrupt::init();
    memory::init();

    // 物理页分配
    for _ in 0..2 {
        let frame_0 = match memory::frame::FRAME_ALLOCATOR.lock().alloc() {
            Result::Ok(frame_tracker) => frame_tracker,
            Result::Err(err) => panic!("{}", err)
        };
        let frame_1 = match memory::frame::FRAME_ALLOCATOR.lock().alloc() {
            Result::Ok(frame_tracker) => frame_tracker,
            Result::Err(err) => panic!("{}", err)
        };
        println!("{} and {}", frame_0.address(), frame_1.address());
    }

    loop{}
}