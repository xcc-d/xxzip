use std::fs::File;
use std::io::BufReader;
use chrono::{Local, TimeZone};
use zip::ZipArchive;
use log::{info, error, warn, debug};
//1
use crate::error::ZipError;
use crate::utils::format_size;
use crate::logger;

/// Lists the contents of a zip file
/// 
/// # Arguments
/// 
/// * `zipfile` - Path to the zip file
/// 
/// # Returns
/// 
/// * `Result<String, ZipError>` - Ok if successful, Err otherwise
pub fn list_zip_contents(zipfile: &str) -> Result<String, ZipError> {
    let file = File::open(zipfile)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;

    // 计算总大小和压缩后大小
    let mut total_size: u64 = 0;
    let mut total_compressed_size: u64 = 0;
    let mut file_count = 0;
    let mut dir_count = 0;

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        total_size += file.size();
        total_compressed_size += file.compressed_size();
        
        if file.name().ends_with('/') {
            dir_count += 1;
        } else {
            file_count += 1;
        }
    }

    // 计算总体压缩率
    let overall_ratio = if total_size > 0 {
        (100.0 * (1.0 - total_compressed_size as f64 / total_size as f64)) as u32
    } else {
        0
    };

    let mut output = String::new();
    output.push_str(&format!("ZIP文件: {}\n", zipfile));
    output.push_str(&format!("总文件数: {}\n", file_count));
    output.push_str(&format!("总目录数: {}\n", dir_count));
    output.push_str(&format!("总大小: {} (压缩后: {}, 压缩率: {}%)\n", 
        format_size(total_size), 
        format_size(total_compressed_size),
        overall_ratio
    ));
    output.push_str("\n");
    
    // 表头
    output.push_str("{:<40} {:>12} {:>12} {:>8} {:<20}\n");
    output.push_str("{:-<40} {:-<12} {:-<12} {:-<8} {:-<20}\n");

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let ratio = if file.size() > 0 {
            (100.0 * (1.0 - file.compressed_size() as f64 / file.size() as f64)) as u32
        } else {
            0
        };

        // 格式化文件名，如果太长则截断
        let name = file.name();
        let display_name = if name.len() > 40 {
            format!("...{}", &name[name.len()-37..])
        } else {
            name.to_string()
        };
        
        // 格式化时间
        let datetime = format_datetime(file.last_modified());
        
        output.push_str(&format!("{:<40} {:>12} {:>12} {:>7}% {:<20}\n",
            display_name,
            format_size(file.size()),
            format_size(file.compressed_size()),
            ratio,
            datetime
        ));
    }

    // 同时记录日志
    logger::info(&format!("列出ZIP文件内容: {}", zipfile));
    
    Ok(output)
}

/// 将MS-DOS时间格式转换为格式化的时间字符串
fn format_datetime(msdos_time: zip::DateTime) -> String {
    // 提取MS-DOS时间的各个部分
    let year = msdos_time.year() as i32;
    let month = msdos_time.month() as u32;
    let day = msdos_time.day() as u32;
    let hour = msdos_time.hour() as u32;
    let minute = msdos_time.minute() as u32;
    let second = msdos_time.second() as u32;
    
    // 使用chrono创建DateTime对象
    if let Some(datetime) = Local.with_ymd_and_hms(year, month, day, hour, minute, second).single() {
        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        // 如果日期无效，返回占位符
        "无效日期".to_string()
    }
} 