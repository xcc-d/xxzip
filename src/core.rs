use std::fs::{self, File};
use std::io::{self, Read, Write, BufReader, BufWriter};
use memmap2::MmapOptions;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

use indicatif::{ProgressBar, ProgressStyle};
use walkdir::WalkDir;
use zip::{ZipArchive, ZipWriter};
use zip::write::FileOptions;

use crate::error::ZipError;

pub fn compress(source_path: &Path, output_path: &Path, level: u32) -> Result<(), ZipError> {
    let start_time = Instant::now();
    let output_file = File::create(output_path)?;
    let writer = BufWriter::new(output_file);
    let mut zip = ZipWriter::new(writer);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(level));

    let (tx, rx) = mpsc::channel();
    let mut total_files = 0;
    let mut total_size = 0;

    // 计算总文件数和大小
    if source_path.is_dir() {
        for entry in WalkDir::new(source_path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total_files += 1;
                total_size += entry.metadata()?.len();
            }
        }
    } else {
        total_files = 1;
        total_size = source_path.metadata()?.len();
    }

    let progress = ProgressBar::new(total_size);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("=>-")
    );

    let handle = std::thread::spawn(move || {
        let mut processed_size = 0;
        while let Ok(size) = rx.recv() {
            processed_size += size;
            progress.set_position(processed_size);
        }
        progress.finish();
    });

    if source_path.is_dir() {
        let base_path = source_path.parent().unwrap_or(Path::new(""));
        for entry in WalkDir::new(source_path) {
            let entry = entry?;
            let path = entry.path();
            let name = path.strip_prefix(base_path)?
                .to_str()
                .ok_or_else(|| ZipError::InvalidPath(path.to_string_lossy().into_owned()))?;

            if path.is_file() {
                zip.start_file(name, options)?;
                // 对大文件使用内存映射（>1GB）
                let file = File::open(path)?;
                let data = if file.metadata()?.len() > 1024 * 1024 * 1024 {
                    let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };
                    zip.write_all(&mmap)?;
                    tx.send(mmap.len()).unwrap_or_default();
                    mmap
                } else {
                    let mut reader = BufReader::new(file);
        let mut buffer = Vec::with_capacity(64 * 1024); // 动态调整缓冲区(64KB-4MB)
                    loop {
                        let bytes_read = reader.read(&mut buffer)?;
                        if bytes_read == 0 {
                            break;
                        }
                        zip.write_all(&buffer[..bytes_read])?;
                        tx.send(bytes_read).unwrap_or_default();
                        
                        // 动态调整缓冲区大小（64KB-4MB）
                        if buffer.len() < 4 * 1024 * 1024 {
                            buffer.resize(buffer.len() * 2, 0);
                        }
                    }
                };
            }
        }
    } else {
        let name = source_path.file_name().unwrap_or_default()
            .to_str()
            .ok_or_else(|| ZipError::InvalidPath(source_path.to_string_lossy().into_owned()))?;
        zip.start_file(name, options)?;
        let mut file = File::open(source_path)?;
        let tx = tx.clone();
        let mut buffer = Vec::with_capacity(64 * 1024); // 初始64KB
        buffer.resize(buffer.capacity(), 0);
        let mut reader = BufReader::with_capacity(buffer.capacity(), file);
        
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            
            // 动态调整缓冲区（64KB-4MB）
            if buffer.capacity() < 4 * 1024 * 1024 {
                buffer.reserve(buffer.capacity() * 2);
                unsafe { buffer.set_len(buffer.capacity()); }
            }
            
            zip.write_all(&buffer[..bytes_read])?;
            tx.send(bytes_read).unwrap_or_default();
        }
    }

    drop(tx);
    handle.join().unwrap();

    zip.finish()?;
    println!("压缩完成！用时：{:.2}秒", start_time.elapsed().as_secs_f64());
    Ok(())
}

pub fn extract(zipfile: &str, output_dir: Option<&Path>, overwrite: bool) -> Result<(), ZipError> {
    let start_time = Instant::now();
    let file = File::open(zipfile)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;

    let output_dir = output_dir.unwrap_or_else(|| Path::new("."));
    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    let total_size: u64 = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok())
        .map(|file| file.size())
        .sum();

    let progress = ProgressBar::new(total_size);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("=>-")
    );

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = output_dir.join(file.mangled_name());

        if let Some(p) = outpath.parent() {
            if !p.exists() {
                fs::create_dir_all(p)?;
            }
        }

        if !overwrite && outpath.exists() {
            println!("文件已存在，跳过：{}", outpath.display());
            continue;
        }

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            let mut outfile = File::create(&outpath)?;
            let mut buffer = vec![0u8; 64 * 1024]; // 64KB初始缓冲区
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                outfile.write_all(&buffer[..bytes_read])?;
                progress.inc(bytes_read as u64);
                
                // 动态调整缓冲区大小（64KB-8MB）
                if buffer.len() < 8 * 1024 * 1024 {
                    buffer.resize(buffer.len() * 2, 0);
                }
            }
        }
    }

    progress.finish();
    println!("解压完成！用时：{:.2}秒", start_time.elapsed().as_secs_f64());
    Ok(())
}

pub fn list_zip_contents(zipfile: &str) -> Result<(), ZipError> {
    let file = File::open(zipfile)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;

    println!("文件数量: {}", archive.len());
    println!("名称\t大小\t压缩后大小\t压缩率");
    println!("-----------------------------------------");

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let ratio = if file.size() > 0 {
            (100.0 * (1.0 - file.compressed_size() as f64 / file.size() as f64)) as u32
        } else {
            0
        };

        println!("{:?}\t{}\t{}\t{}%",
            file.name(),
            file.size(),
            file.compressed_size(),
            ratio
        );
    }

    Ok(())
}
