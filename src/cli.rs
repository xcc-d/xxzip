use std::path::{Path, PathBuf};
use clap::{Parser, Subcommand};
// 1    
use crate::compress;
use crate::extract;
use crate::list;
use crate::error::ZipError;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// 启动GUI模式
    #[arg(short, long)]
    pub gui: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 压缩文件或目录
    Compress {
        /// 要压缩的文件或目录路径
        #[arg(required = true)]
        source: String,
        
        /// 输出的ZIP文件路径
        #[arg(short, long)]
        output: Option<String>,
        
        /// 压缩级别 (0-9)，0表示不压缩，9表示最大压缩
        #[arg(short, long, default_value_t = 6)]
        level: u32,
    },
    
    /// 解压缩ZIP文件
    Extract {
        /// ZIP文件路径
        #[arg(required = true)]
        zipfile: String,
        
        /// 解压缩目标目录
        #[arg(short, long)]
        output_dir: Option<String>,
        
        /// 是否覆盖已存在的文件
        #[arg(short, long, default_value_t = false)]
        overwrite: bool,
    },
    
    /// 列出ZIP文件内容
    List {
        /// ZIP文件路径
        #[arg(required = true)]
        zipfile: String,
    },
}

/// Handles CLI commands
/// 
/// # Arguments
/// 
/// * `cli` - The parsed CLI arguments
/// 
/// # Returns
/// 
/// * `Result<(), ZipError>` - Ok if successful, Err otherwise
pub fn handle_command(cli: &Cli) -> Result<(), ZipError> {
    match &cli.command {
        Some(Commands::Compress { source, output, level }) => {
            let source_path = Path::new(source);
            let output_path = match output {
                Some(path) => PathBuf::from(path),
                None => {
                    let mut path = PathBuf::from(source);
                    if path.is_dir() {
                        path = path.with_file_name(format!("{}.zip", path.file_name().unwrap_or_default().to_string_lossy()));
                    } else {
                        path = path.with_extension("zip");
                    }
                    path
                }
            };
            
            compress::compress(source_path, &output_path, *level)?;
        }
        
        Some(Commands::Extract { zipfile, output_dir, overwrite }) => {
            let output_path = output_dir.as_ref().map(Path::new);
            extract::extract(zipfile, output_path, *overwrite)?;
        }
        
        Some(Commands::List { zipfile }) => {
            list::list_zip_contents(zipfile)?;
        }
        
        None => {
            // No command provided, GUI mode will be handled elsewhere
        }
    }
    
    Ok(())
} 