use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::SystemTime;
use chrono::{DateTime, Local};
//1
// 全局日志文件句柄
lazy_static::lazy_static! {
    static ref LOG_FILE: Mutex<Option<File>> = Mutex::new(None);
}

// 初始化日志系统
pub fn init() -> Result<(), std::io::Error> {
    let log_path = get_log_path();
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    
    let mut log_file = LOG_FILE.lock().unwrap();
    *log_file = Some(file);
    
    Ok(())
}

// 获取日志文件路径
fn get_log_path() -> std::path::PathBuf {
    let exe_path = std::env::current_exe().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let exe_dir = exe_path.parent().unwrap_or_else(|| Path::new("."));
    exe_dir.join("zip_tool.log")
}

// 写入日志
pub fn log(level: &str, message: &str) {
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(file) = guard.as_mut() {
            let now: DateTime<Local> = SystemTime::now().into();
            let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
            
            let log_line = format!("[{}] {}: {}\n", timestamp, level, message);
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
    }
}

// 公共日志函数，可以直接调用
pub fn info(message: &str) {
    log("INFO", message);
}

pub fn error(message: &str) {
    log("ERROR", message);
}

pub fn debug(message: &str) {
    #[cfg(debug_assertions)]
    log("DEBUG", message);
}

// 添加格式化版本的日志函数
pub fn info_fmt(args: std::fmt::Arguments) {
    info(&format!("{}", args));
}

pub fn error_fmt(args: std::fmt::Arguments) {
    error(&format!("{}", args));
}

pub fn debug_fmt(args: std::fmt::Arguments) {
    #[cfg(debug_assertions)]
    debug(&format!("{}", args));
}

// 宏定义
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        crate::logger::info(&format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        crate::logger::error(&format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        crate::logger::debug(&format!($($arg)*));
    };
} 