use std::fs::File;
use std::io::{Read, Write, BufReader};
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use walkdir::WalkDir;
use zip::ZipWriter;
use zip::write::FileOptions;

use crate::error::ZipError;
use crate::utils::create_progress_bar;
use log::{info, error, debug, warn};

/// Compresses a file or directory into a zip file
/// 
/// # Arguments
/// 
/// * `source_path` - The path to the file or directory to compress
/// * `output_path` - The path where the zip file will be created
/// * `level` - Compression level (0-9)
/// 
/// # Returns
/// 
/// * `Result<(), ZipError>` - Ok if successful, Err otherwise
pub fn compress(source_path: &Path, output_path: &Path, level: u32) -> Result<(), ZipError> {
    let start_time = Instant::now();
    let output_file = File::create(output_path)?;
    let writer = std::io::BufWriter::new(output_file);
    let mut zip = ZipWriter::new(writer);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(level as i32));

    let (tx, rx) = mpsc::channel();
    let mut total_size = 0;

    // Calculate total files and size
    if source_path.is_dir() {
        for entry in WalkDir::new(source_path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total_size += entry.metadata()?.len();
            }
        }
    } else {
        total_size = source_path.metadata()?.len();
    }

    let progress = create_progress_bar(total_size);

    let handle = std::thread::spawn(move || {
        let mut processed_size = 0;
        while let Ok(size) = rx.recv() {
            processed_size += size;
            progress.set_position(processed_size);
        }
        progress.finish();
    });

    if source_path.is_dir() {
        compress_directory(source_path, &mut zip, options, &tx)?;
    } else {
        compress_file(source_path, &mut zip, options, &tx)?;
    }

    drop(tx);
    handle.join().unwrap();

    zip.finish()?;
    info!("压缩完成！用时：{:.2}秒", start_time.elapsed().as_secs_f64());
    Ok(())
}

fn compress_directory(
    source_path: &Path, 
    zip: &mut ZipWriter<std::io::BufWriter<File>>, 
    options: FileOptions, 
    tx: &mpsc::Sender<u64>
) -> Result<(), ZipError> {
    let base_path = source_path.parent().unwrap_or(Path::new(""));
    
    for entry in WalkDir::new(source_path) {
        let entry = entry?;
        let path = entry.path();
        let name = path.strip_prefix(base_path)?
            .to_str()
            .ok_or_else(|| ZipError::InvalidPath(path.to_string_lossy().into_owned()))?;

        if path.is_file() {
            zip.start_file(name, options)?;
            // 使用内存映射的阈值从1GB降低到100MB，更合理地使用内存映射
            let file = File::open(path)?;
            let file_size = file.metadata()?.len();
            
            if file_size > 100 * 1024 * 1024 {
                // 对于大文件使用内存映射
                match unsafe { memmap2::MmapOptions::new().map(&file) } {
                    Ok(mmap) => {
                        zip.write_all(&mmap)?;
                        if let Err(e) = tx.send(mmap.len() as u64) {
                            warn!("无法发送进度更新: {}", e);
                        }
                    },
                    Err(e) => {
                        // 如果内存映射失败，回退到标准读取
                        warn!("内存映射失败，使用标准读取: {}", e);
                        read_and_write_file(&file, zip, tx, file_size)?;
                    }
                }
            } else {
                // 对于小文件使用标准读取
                read_and_write_file(&file, zip, tx, file_size)?;
            }
        }
    }
    
    Ok(())
}

// 提取公共的文件读写逻辑到单独的函数
fn read_and_write_file(
    file: &File,
    zip: &mut ZipWriter<std::io::BufWriter<File>>,
    tx: &mpsc::Sender<u64>,
    file_size: u64
) -> Result<(), ZipError> {
    // 根据文件大小选择初始缓冲区大小
    let initial_buffer_size = if file_size < 1024 * 1024 {
        // 小于1MB的文件使用32KB缓冲区
        32 * 1024
    } else {
        // 大于1MB的文件使用64KB缓冲区
        64 * 1024
    };
    
    // 设置缓冲区大小上限为2MB，避免过度消耗内存
    const MAX_BUFFER_SIZE: usize = 2 * 1024 * 1024;
    
    let mut buffer = vec![0u8; initial_buffer_size];
    let mut reader = BufReader::with_capacity(initial_buffer_size, file);
    
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        
        zip.write_all(&buffer[..bytes_read])?;
        if let Err(e) = tx.send(bytes_read as u64) {
            warn!("无法发送进度更新: {}", e);
        }
        
        // 动态调整缓冲区大小，但不超过上限
        if buffer.len() < MAX_BUFFER_SIZE && bytes_read == buffer.len() {
            let new_size = std::cmp::min(buffer.len() * 2, MAX_BUFFER_SIZE);
            buffer.resize(new_size, 0);
        }
    }
    
    Ok(())
}

fn compress_file(
    source_path: &Path, 
    zip: &mut ZipWriter<std::io::BufWriter<File>>, 
    options: FileOptions, 
    tx: &mpsc::Sender<u64>
) -> Result<(), ZipError> {
    let name = source_path.file_name().unwrap_or_default()
        .to_str()
        .ok_or_else(|| ZipError::InvalidPath(source_path.to_string_lossy().into_owned()))?;
    
    zip.start_file(name, options)?;
    let file = File::open(source_path)?;
    let file_size = file.metadata()?.len();
    
    // 使用内存映射的阈值从1GB降低到100MB
    if file_size > 100 * 1024 * 1024 {
        match unsafe { memmap2::MmapOptions::new().map(&file) } {
            Ok(mmap) => {
                zip.write_all(&mmap)?;
                if let Err(e) = tx.send(mmap.len() as u64) {
                    warn!("无法发送进度更新: {}", e);
                }
            },
            Err(e) => {
                // 如果内存映射失败，回退到标准读取
                warn!("内存映射失败，使用标准读取: {}", e);
                read_and_write_file(&file, zip, tx, file_size)?;
            }
        }
    } else {
        read_and_write_file(&file, zip, tx, file_size)?;
    }
    
    Ok(())
} 