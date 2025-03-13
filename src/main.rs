use std::fs::{self, File};
use std::io::{self, Read, Write, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::process;
use std::sync::mpsc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use walkdir::WalkDir;
use zip::{ZipArchive, ZipWriter};
use zip::write::FileOptions;

// GUI相关导入
#[cfg(feature = "gui")]
use eframe::egui;
#[cfg(feature = "gui")]
use egui::{Color32, RichText};
#[cfg(feature = "gui")]
use rfd::FileDialog;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// 启动GUI模式
    #[arg(short, long)]
    gui: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
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

// 添加日志文件支持
fn log_error(error: &str) {
    let log_path = Path::new("error_log.txt");
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path) {
        let _ = writeln!(file, "{}: {}", chrono::Local::now(), error);
    }
}

fn main() {
    if let Err(e) = run() {
        let error_msg = format!("程序错误: {}", e);
        log_error(&error_msg);
        // 在控制台显示错误
        eprintln!("{}", error_msg);
        // 等待用户输入后再退出
        println!("\n按回车键退出...");
        let mut input = String::new();
        let _ = io::stdin().read_line(&mut input);
        process::exit(1);
    }
}

fn run() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();
    
    // 如果指定了--gui参数或没有指定子命令，启动GUI模式
    if cli.gui || cli.command.is_none() {
        #[cfg(feature = "gui")]
        {
            let app = ZipToolApp::default();
            let native_options = eframe::NativeOptions {
                initial_window_size: Some(egui::vec2(600.0, 500.0)),
                ..Default::default()
            };
            
            // 改进错误处理，确保在GUI模式下也能显示错误信息
            match eframe::run_native(
                "ZIP工具",
                native_options,
                Box::new(|_cc| Box::new(app)),
            ) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    let error_msg = format!("GUI启动错误: {}", e);
                    log_error(&error_msg);
                    eprintln!("{}", error_msg);
                    
                    // 在GUI模式下，确保用户能看到错误信息
                    // 检查是否有控制台附加（通过尝试获取控制台窗口句柄）
                    let has_console = atty::is(atty::Stream::Stdin);
                    
                    if !has_console {
                        // 如果是通过双击启动的（没有终端），使用消息框显示错误
                        #[cfg(target_os = "windows")]
                        {
                            use std::ptr::null_mut;
                            use winapi::um::winuser::{MessageBoxW, MB_ICONERROR, MB_OK};
                            
                            let title: Vec<u16> = "错误".encode_utf16().chain(std::iter::once(0)).collect();
                            let msg: Vec<u16> = error_msg.encode_utf16().chain(std::iter::once(0)).collect();
                            
                            unsafe {
                                MessageBoxW(
                                    null_mut(),
                                    msg.as_ptr(),
                                    title.as_ptr(),
                                    MB_ICONERROR | MB_OK
                                );
                            }
                        }
                        
                        #[cfg(not(target_os = "windows"))]
                        {
                            // 非Windows系统，尝试使用其他方式显示错误
                            if let Err(e) = std::process::Command::new("zenity")
                                .args(["--error", "--text", &error_msg])
                                .output() {
                                log_error(&format!("无法显示错误对话框: {}", e));
                            }
                        }
                    } else {
                        // 在终端模式下，等待用户输入
                        println!("\n按回车键退出...");
                        let mut input = String::new();
                        let _ = io::stdin().read_line(&mut input);
                    }
                    
                    return Err(anyhow::anyhow!(error_msg));
                }
            }
        }
        
        #[cfg(not(feature = "gui"))]
        {
            return Err(anyhow::anyhow!("GUI模式未启用。请使用 --features gui 重新编译，或使用命令行模式。"));
        }
    }
    
    // 命令行模式
    match cli.command.unwrap() {
        Commands::Compress { source, output, level } => {
            let source_path = Path::new(&source);
            let output_path = match output {
                Some(path) => PathBuf::from(path),
                None => {
                    let mut path = source_path.to_path_buf();
                    if path.is_dir() {
                        path = path.with_file_name(format!("{}.zip", path.file_name().unwrap().to_string_lossy()));
                    } else {
                        path = path.with_extension("zip");
                    }
                    path
                }
            };
            
            compress(source_path, &output_path, level)?;
        },
        Commands::Extract { zipfile, output_dir, overwrite } => {
            let output_path = match output_dir {
                Some(path) => PathBuf::from(path),
                None => {
                    let zip_path = Path::new(&zipfile);
                    let file_stem = zip_path.file_stem().unwrap_or_default();
                    PathBuf::from(file_stem)
                }
            };
            
            extract(Path::new(&zipfile), &output_path, overwrite)?;
        },
        Commands::List { zipfile } => {
            list_contents(Path::new(&zipfile))?;
        },
    }
    
    Ok(())
}

/// 压缩文件或目录到ZIP文件
fn compress(source: &Path, dest: &Path, level: u32) -> Result<()> {
    let start_time = Instant::now();
    println!("正在压缩 {} 到 {}", source.display(), dest.display());
    
    // 确保压缩级别在有效范围内
    let compression_level = if level > 9 { 9 } else { level };
    let compression = match compression_level {
        0 => zip::CompressionMethod::Stored,
        _ => zip::CompressionMethod::Deflated,
    };
    
    let file = File::create(dest).context("无法创建ZIP文件")?;
    let buf_writer = BufWriter::new(file);
    let mut zip = ZipWriter::new(buf_writer);
    
    let options = FileOptions::default()
        .compression_method(compression)
        .unix_permissions(0o755)
        .compression_level(Some(compression_level as i32));
    
    let mut files_to_compress = Vec::new();
    
    if source.is_file() {
        // 单个文件
        files_to_compress.push((
            source.to_path_buf(),
            source.file_name().unwrap().to_string_lossy().to_string(),
        ));
    } else if source.is_dir() {
        // 目录
        let base_path = source.parent().unwrap_or(Path::new(""));
        for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let relative_path = path.strip_prefix(base_path).unwrap();
                files_to_compress.push((
                    path.to_path_buf(),
                    relative_path.to_string_lossy().to_string(),
                ));
            }
        }
    } else {
        return Err(anyhow::anyhow!("源路径不存在或无法访问"));
    }
    
    let progress_bar = ProgressBar::new(files_to_compress.len() as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    
    for (file_path, name_in_zip) in files_to_compress {
        progress_bar.set_message(format!("正在添加: {}", name_in_zip));
        
        if file_path.is_file() {
            zip.start_file(&name_in_zip, options)?;
            let mut file = File::open(&file_path).context(format!("无法打开文件: {}", file_path.display()))?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        }
        
        progress_bar.inc(1);
    }
    
    zip.finish()?;
    progress_bar.finish_with_message("压缩完成");
    
    let elapsed = start_time.elapsed();
    println!("压缩完成，用时: {:.2}秒", elapsed.as_secs_f64());
    
    Ok(())
}

/// 解压缩ZIP文件到指定目录
fn extract(zip_path: &Path, output_dir: &Path, overwrite: bool) -> Result<()> {
    let start_time = Instant::now();
    println!("正在解压 {} 到 {}", zip_path.display(), output_dir.display());
    
    // 确保输出目录存在
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).context("无法创建输出目录")?;
    }
    
    let file = File::open(zip_path).context("无法打开ZIP文件")?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader).context("无法读取ZIP文件")?;
    
    let total_files = archive.len();
    let progress_bar = ProgressBar::new(total_files as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    
    for i in 0..total_files {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => output_dir.join(path),
            None => continue,
        };
        
        progress_bar.set_message(format!("正在解压: {}", outpath.display()));
        
        if file.name().ends_with('/') {
            // 目录
            fs::create_dir_all(&outpath).context(format!("无法创建目录: {}", outpath.display()))?;
        } else {
            // 文件
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).context(format!("无法创建父目录: {}", parent.display()))?;
                }
            }
            
            if outpath.exists() && !overwrite {
                progress_bar.println(format!("跳过已存在的文件: {}", outpath.display()));
                progress_bar.inc(1);
                continue;
            }
            
            let mut outfile = File::create(&outpath).context(format!("无法创建文件: {}", outpath.display()))?;
            io::copy(&mut file, &mut outfile)?;
            
            // 设置文件权限（仅在类Unix系统上有效）
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }
        }
        
        progress_bar.inc(1);
    }
    
    progress_bar.finish_with_message("解压完成");
    
    let elapsed = start_time.elapsed();
    println!("解压完成，用时: {:.2}秒", elapsed.as_secs_f64());
    
    Ok(())
}

/// 列出ZIP文件内容
fn list_contents(zip_path: &Path) -> Result<()> {
    println!("ZIP文件: {}", zip_path.display());
    
    let file = File::open(zip_path).context("无法打开ZIP文件")?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader).context("无法读取ZIP文件")?;
    
    println!("文件数量: {}", archive.len());
    println!("----------------------------------------");
    println!("  大小\t压缩后\t比例\t文件名");
    println!("----------------------------------------");
    
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let ratio = if file.size() > 0 {
            (file.compressed_size() as f64 / file.size() as f64 * 100.0) as u32
        } else {
            0
        };
        
        println!("{:8}\t{:8}\t{}%\t{}", 
            file.size(), 
            file.compressed_size(),
            ratio,
            file.name()
        );
    }
    
    Ok(())
}

// GUI实现
#[cfg(feature = "gui")]
#[derive(PartialEq, Eq, Clone, Copy)]
enum Operation {
    Compress,
    Extract,
    List,
}

#[cfg(feature = "gui")]
enum OperationState {
    Idle,
    InProgress,
    Done(String),
    Error(String),
}

#[cfg(feature = "gui")]
struct ZipToolApp {
    operation: Operation,
    source_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    compression_level: u32,
    overwrite: bool,
    operation_state: OperationState,
    dark_mode: bool,
    result_receiver: Option<mpsc::Receiver<String>>,
    fonts_loaded: bool,
}

#[cfg(feature = "gui")]
impl Default for ZipToolApp {
    fn default() -> Self {
        Self {
            operation: Operation::Compress,
            source_path: None,
            output_path: None,
            compression_level: 6,
            overwrite: false,
            operation_state: OperationState::Idle,
            dark_mode: false,
            result_receiver: None,
            fonts_loaded: false,
        }
    }
}

#[cfg(feature = "gui")]
impl eframe::App for ZipToolApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 第一次运行时加载字体
        if !self.fonts_loaded {
            let mut fonts = egui::FontDefinitions::default();
            let font_path = Path::new("assets/SourceHanSerif-VF.otf.ttc");
            if let Ok(data) = fs::read(font_path) {
                let font_data = egui::FontData::from_owned(data);
                fonts.font_data.insert("source-han-serif".to_owned(), font_data);
                fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
                    .insert(0, "source-han-serif".to_owned());
                fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap()
                    .push("source-han-serif".to_owned());
            } else {
                log_error("无法加载字体文件，将使用系统默认字体");
            }
            ctx.set_fonts(fonts);
            self.fonts_loaded = true;
        }

        // 检查异步操作的结果
        if let Some(receiver) = &self.result_receiver {
            if let Ok(msg) = receiver.try_recv() {
                // 更新操作状态
                if msg.contains("失败") {
                    self.operation_state = OperationState::Error(msg);
                } else {
                    self.operation_state = OperationState::Done(msg);
                }
                // 清除接收器
                self.result_receiver = None;
            }
        }

        // 设置主题
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("ZIP工具");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if self.dark_mode { "☀️ 亮色模式" } else { "🌙 夜间模式" }).clicked() {
                        self.dark_mode = !self.dark_mode;
                    }
                });
            });
            
            ui.separator();
            
            // 操作选择
            ui.horizontal(|ui| {
                ui.label("操作:");
                ui.radio_value(&mut self.operation, Operation::Compress, "压缩");
                ui.radio_value(&mut self.operation, Operation::Extract, "解压缩");
                ui.radio_value(&mut self.operation, Operation::List, "列出内容");
            });
            
            ui.separator();
            
            // 源路径选择
            ui.horizontal(|ui| {
                let label = match self.operation {
                    Operation::Compress => "源文件/目录:",
                    Operation::Extract | Operation::List => "ZIP文件:",
                };
                ui.label(label);
                
                let path_text = match &self.source_path {
                    Some(path) => path.to_string_lossy().to_string(),
                    None => "未选择".to_string(),
                };
                
                ui.text_edit_singleline(&mut path_text.clone());
                
                if ui.button("浏览...").clicked() {
                    self.select_source_path();
                }
            });
            
            // 输出路径选择（压缩和解压缩时显示）
            match self.operation {
                Operation::Compress | Operation::Extract => {
                    ui.horizontal(|ui| {
                        let label = match self.operation {
                            Operation::Compress => "输出ZIP文件:",
                            Operation::Extract => "解压目录:",
                            _ => "",
                        };
                        ui.label(label);
                        
                        let path_text = match &self.output_path {
                            Some(path) => path.to_string_lossy().to_string(),
                            None => "未选择".to_string(),
                        };
                        
                        ui.text_edit_singleline(&mut path_text.clone());
                        
                        if ui.button("浏览...").clicked() {
                            self.select_output_path();
                        }
                    });
                },
                _ => {},
            }
            
            // 压缩级别（仅压缩时显示）
            if let Operation::Compress = self.operation {
                ui.horizontal(|ui| {
                    ui.label("压缩级别:");
                    ui.add(egui::Slider::new(&mut self.compression_level, 0..=9)
                        .text("0=不压缩, 9=最大压缩"));
                });
            }
            
            // 覆盖选项（仅解压缩时显示）
            if let Operation::Extract = self.operation {
                ui.checkbox(&mut self.overwrite, "覆盖已存在的文件");
            }
            
            ui.separator();
            
            // 执行按钮
            let button_text = match self.operation {
                Operation::Compress => "压缩",
                Operation::Extract => "解压缩",
                Operation::List => "列出内容",
            };
            
            let button_enabled = match self.operation {
                Operation::Compress => self.source_path.is_some() && self.output_path.is_some(),
                Operation::Extract => self.source_path.is_some() && self.output_path.is_some(),
                Operation::List => self.source_path.is_some(),
            };
            
            if ui.add_enabled(button_enabled, egui::Button::new(button_text)).clicked() {
                self.execute_operation();
            }
            
            ui.separator();
            
            // 显示操作状态和结果
            match &self.operation_state {
                OperationState::Idle => {
                    ui.label("准备就绪");
                },
                OperationState::InProgress => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("正在处理...");
                    });
                },
                OperationState::Done(output) => {
                    ui.label(RichText::new("操作完成").color(Color32::GREEN));
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        ui.monospace(output);
                    });
                },
                OperationState::Error(err) => {
                    ui.label(RichText::new("操作失败").color(Color32::RED));
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        ui.monospace(err);
                    });
                },
            }
        });
    }
}

#[cfg(feature = "gui")]
impl ZipToolApp {
    fn select_source_path(&mut self) {
        match self.operation {
            Operation::Compress => {
                // 选择文件或目录
                if let Some(path) = FileDialog::new()
                    .pick_folder() {
                    self.source_path = Some(path);
                    // 自动设置输出路径
                    if let Some(source) = &self.source_path {
                        let mut output = source.clone();
                        if output.is_dir() {
                            output = output.with_file_name(format!("{}.zip", output.file_name().unwrap().to_string_lossy()));
                        } else {
                            output = output.with_extension("zip");
                        }
                        self.output_path = Some(output);
                    }
                }
            },
            Operation::Extract | Operation::List => {
                // 只选择ZIP文件
                if let Some(path) = FileDialog::new()
                    .add_filter("ZIP文件", &["zip"])
                    .pick_file() {
                    self.source_path = Some(path);
                    // 自动设置解压目录
                    if let Some(source) = &self.source_path {
                        if let Some(stem) = source.file_stem() {
                            let mut output = source.parent().unwrap().to_path_buf();
                            output.push(stem);
                            self.output_path = Some(output);
                        }
                    }
                }
            },
        }
    }
    
    fn select_output_path(&mut self) {
        match self.operation {
            Operation::Compress => {
                // 选择保存ZIP文件的位置
                if let Some(path) = FileDialog::new()
                    .add_filter("ZIP文件", &["zip"])
                    .set_file_name("archive.zip")
                    .save_file() {
                    self.output_path = Some(path);
                }
            },
            Operation::Extract => {
                // 选择解压目录
                if let Some(path) = FileDialog::new()
                    .pick_folder() {
                    self.output_path = Some(path);
                }
            },
            Operation::List => {
                // 列出内容不需要输出路径
            },
        }
    }
    
    fn execute_operation(&mut self) {
        self.operation_state = OperationState::InProgress;
        
        // 创建通道用于线程间通信
        let (tx, rx) = mpsc::channel();
        
        match self.operation {
            Operation::Compress => {
                if let (Some(source), Some(output)) = (&self.source_path, &self.output_path) {
                    let source = source.clone();
                    let output = output.clone();
                    let level = self.compression_level;
                    
                    thread::spawn(move || {
                        match compress(&source, &output, level) {
                            Ok(_) => tx.send(format!("成功压缩 {} 到 {}", source.display(), output.display())).unwrap(),
                            Err(e) => tx.send(format!("压缩失败: {}", e)).unwrap(),
                        }
                    });
                }
            },
            Operation::Extract => {
                if let (Some(source), Some(output)) = (&self.source_path, &self.output_path) {
                    let source = source.clone();
                    let output = output.clone();
                    let overwrite = self.overwrite;
                    
                    thread::spawn(move || {
                        match extract(&source, &output, overwrite) {
                            Ok(_) => tx.send(format!("成功解压 {} 到 {}", source.display(), output.display())).unwrap(),
                            Err(e) => tx.send(format!("解压失败: {}", e)).unwrap(),
                        }
                    });
                }
            },
            Operation::List => {
                if let Some(source) = &self.source_path {
                    let source = source.clone();
                    
                    thread::spawn(move || {
                        match list_contents(&source) {
                            Ok(_) => tx.send(format!("成功列出 {} 的内容", source.display())).unwrap(),
                            Err(e) => tx.send(format!("列出内容失败: {}", e)).unwrap(),
                        }
                    });
                }
            },
        }
        
        // 存储接收器以便在UI更新时检查结果
        self.result_receiver = Some(rx);
    }
}
use std::thread;

// 加载应用图标
fn load_icon_data() -> Option<eframe::IconData> {
    let icon_path = std::path::PathBuf::from(std::env::current_exe().unwrap_or_default())
        .parent()
        .unwrap_or(&std::path::PathBuf::new())
        .join("assets")
        .join("p.png");

    if let Ok(image_data) = std::fs::read(&icon_path) {
        if let Ok(image) = image::load_from_memory(&image_data) {
            let rgba = image.into_rgba8();
            let (width, height) = rgba.dimensions();
            return Some(eframe::IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            });
        }
    }
    eprintln!("警告: 无法加载图标文件 {}: 将使用默认图标", icon_path.display());
    None
}
