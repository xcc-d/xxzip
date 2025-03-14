use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Instant;
//1
use indicatif::ProgressBar;
use zip::ZipArchive;

use crate::error::ZipError;
use crate::utils::create_progress_bar;
use log::{info, error, warn, debug};
use simplelog::{WriteLogger, Config, LevelFilter};

/// Extracts a zip file to a directory
/// 
/// # Arguments
/// 
/// * `zipfile` - Path to the zip file
/// * `output_dir` - Directory to extract to (defaults to current directory if None)
/// * `overwrite` - Whether to overwrite existing files
/// 
/// # Returns
/// 
/// * `Result<(), ZipError>` - Ok if successful, Err otherwise
pub fn extract(zipfile: &str, output_dir: Option<&Path>, overwrite: bool) -> Result<(), ZipError> {
    let start_time = Instant::now();
    let file = File::open(zipfile)?;
    let mut archive = ZipArchive::new(std::io::BufReader::new(file))?;

    let output_dir = output_dir.unwrap_or_else(|| Path::new("."));
    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    // 计算总大小
    let mut total_size: u64 = 0;
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            total_size += file.size();
        }
    }

    let progress = create_progress_bar(total_size);
    let mut extracted_files = 0;
    let total_files = archive.len();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = output_dir.join(file.mangled_name());

        if let Some(p) = outpath.parent() {
            if !p.exists() {
                fs::create_dir_all(p)?;
            }
        }

        if !overwrite && outpath.exists() {
            info!("文件已存在，跳过：{}", outpath.display());
            // 更新进度条，即使跳过文件
            progress.inc(file.size());
            continue;
        }

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            extract_file(&mut file, &outpath, &progress)?;
        }
        
        extracted_files += 1;
        if extracted_files % 10 == 0 || extracted_files == total_files {
            info!("已解压 {}/{} 个文件", extracted_files, total_files);
        }
    }

    progress.finish();
    info!("解压完成！用时：{:.2}秒", start_time.elapsed().as_secs_f64());
    Ok(())
}

fn extract_file(
    file: &mut zip::read::ZipFile, 
    outpath: &Path, 
    progress: &ProgressBar
) -> Result<(), ZipError> {
    let mut outfile = File::create(outpath)?;
    
    // 根据文件大小选择初始缓冲区大小
    let initial_buffer_size = if file.size() < 1024 * 1024 {
        // 小于1MB的文件使用32KB缓冲区
        32 * 1024
    } else {
        // 大于1MB的文件使用64KB缓冲区
        64 * 1024
    };
    
    // 设置缓冲区大小上限为2MB，避免过度消耗内存
    const MAX_BUFFER_SIZE: usize = 2 * 1024 * 1024;
    
    let mut buffer = vec![0u8; initial_buffer_size];
    
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        outfile.write_all(&buffer[..bytes_read])?;
        progress.inc(bytes_read as u64);
        
        // 动态调整缓冲区大小，但不超过上限
        if buffer.len() < MAX_BUFFER_SIZE && bytes_read == buffer.len() {
            let new_size = std::cmp::min(buffer.len() * 2, MAX_BUFFER_SIZE);
            buffer.resize(new_size, 0);
        }
    }
    
    // 确保文件被完全写入磁盘
    outfile.flush()?;
    
    Ok(())
} 