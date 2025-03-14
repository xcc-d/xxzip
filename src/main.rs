mod compress;
mod extract;
mod list;
mod error;
mod utils;
mod gui;
mod logger;
//1
use log::{info, error, warn, debug};
use simplelog::{WriteLogger, Config, LevelFilter};
use std::fs::File;

#[macro_use]
extern crate lazy_static;

fn main() {
    // 初始化日志系统
    let log_path = std::env::current_exe()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("zip_tool.log");
    
    if let Ok(file) = File::create(log_path) {
        if let Err(e) = WriteLogger::init(LevelFilter::Info, Config::default(), file) {
            show_error_message(&format!("无法初始化日志系统: {}", e));
        }
    } else {
        show_error_message("无法创建日志文件");
    }
    
    info!("应用程序启动");
    
    if let Err(e) = gui::run_gui() {
        logger::error(&format!("GUI启动失败: {}", e));
        show_error_message(&format!("GUI启动失败: {}", e));
        std::process::exit(1);
    }
}

// 显示错误消息的辅助函数
#[cfg(windows)]
fn show_error_message(message: &str) {
    use std::ffi::CString;
    use std::ptr::null_mut;
    
    // 将Rust字符串转换为C字符串
    let error_msg = match CString::new(message) {
        Ok(s) => s,
        Err(_) => CString::new("发生错误").unwrap(),
    };
    
    let title = CString::new("错误").unwrap_or_else(|_| CString::new("").unwrap());
    
    unsafe {
        winapi::um::winuser::MessageBoxA(
            null_mut(),
            error_msg.as_ptr(),
            title.as_ptr(),
            winapi::um::winuser::MB_OK | winapi::um::winuser::MB_ICONERROR
        );
    }
}

#[cfg(not(windows))]
fn show_error_message(message: &str) {
    eprintln!("{}", message);
}
