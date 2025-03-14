use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
//1
use eframe::egui;
use egui::Color32;
use rfd::FileDialog;

use crate::compress;
use crate::extract;
use crate::list;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Operation {
    Compress,
    Extract,
    List,
}

pub enum OperationState {
    Idle,
    InProgress,
    Done(String),
    Error(String),
}

pub struct ZipToolApp {
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

impl Default for ZipToolApp {
    fn default() -> Self {
        Self {
            operation: Operation::Compress,
            source_path: None,
            output_path: None,
            compression_level: 6,
            overwrite: false,
            operation_state: OperationState::Idle,
            dark_mode: true,
            result_receiver: None,
            fonts_loaded: false,
        }
    }
}

impl eframe::App for ZipToolApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.fonts_loaded {
            self.fonts_loaded = true;
            let fonts = egui::FontDefinitions::default();
            // Add custom fonts here if needed
            ctx.set_fonts(fonts);
        }

        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        // Check for operation results
        if let Some(receiver) = &self.result_receiver {
            if let Ok(result) = receiver.try_recv() {
                self.operation_state = OperationState::Done(result);
                self.result_receiver = None;
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("文件", |ui| {
                    if ui.button("退出").clicked() {
                        frame.close();
                    }
                });
                ui.menu_button("主题", |ui| {
                    if ui.button(if self.dark_mode { "亮色主题" } else { "暗色主题" }).clicked() {
                        self.dark_mode = !self.dark_mode;
                        ui.close_menu();
                    }
                });
                ui.menu_button("帮助", |ui| {
                    if ui.button("关于").clicked() {
                        // Show about dialog
                        ui.close_menu();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ZIP工具");
            ui.separator();

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.operation, Operation::Compress, "压缩");
                ui.selectable_value(&mut self.operation, Operation::Extract, "解压");
                ui.selectable_value(&mut self.operation, Operation::List, "列表");
            });

            ui.separator();

            match self.operation {
                Operation::Compress => {
                    ui.horizontal(|ui| {
                        ui.label("源文件/目录:");
                        if let Some(path) = &self.source_path {
                            ui.label(path.to_string_lossy().to_string());
                        } else {
                            ui.label("未选择");
                        }
                        if ui.button("浏览...").clicked() {
                            self.select_source_path();
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("输出文件:");
                        if let Some(path) = &self.output_path {
                            ui.label(path.to_string_lossy().to_string());
                        } else {
                            ui.label("未选择");
                        }
                        if ui.button("浏览...").clicked() {
                            self.select_output_path();
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("压缩级别:");
                        ui.add(egui::Slider::new(&mut self.compression_level, 0..=9));
                    });
                }
                Operation::Extract => {
                    ui.horizontal(|ui| {
                        ui.label("ZIP文件:");
                        if let Some(path) = &self.source_path {
                            ui.label(path.to_string_lossy().to_string());
                        } else {
                            ui.label("未选择");
                        }
                        if ui.button("浏览...").clicked() {
                            self.select_source_path();
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("输出目录:");
                        if let Some(path) = &self.output_path {
                            ui.label(path.to_string_lossy().to_string());
                        } else {
                            ui.label("未选择");
                        }
                        if ui.button("浏览...").clicked() {
                            self.select_output_path();
                        }
                    });

                    ui.checkbox(&mut self.overwrite, "覆盖已存在的文件");
                }
                Operation::List => {
                    ui.horizontal(|ui| {
                        ui.label("ZIP文件:");
                        if let Some(path) = &self.source_path {
                            ui.label(path.to_string_lossy().to_string());
                        } else {
                            ui.label("未选择");
                        }
                        if ui.button("浏览...").clicked() {
                            self.select_source_path();
                        }
                    });
                }
            }

            ui.separator();

            match &self.operation_state {
                OperationState::Idle => {
                    if ui.button("执行").clicked() {
                        self.execute_operation();
                    }
                }
                OperationState::InProgress => {
                    ui.spinner();
                    ui.label("处理中...");
                }
                OperationState::Done(message) => {
                    ui.colored_label(Color32::GREEN, message);
                    if ui.button("确定").clicked() {
                        self.operation_state = OperationState::Idle;
                    }
                }
                OperationState::Error(message) => {
                    ui.colored_label(Color32::RED, message);
                    if ui.button("确定").clicked() {
                        self.operation_state = OperationState::Idle;
                    }
                }
            }
        });
    }
}

impl ZipToolApp {
    fn select_source_path(&mut self) {
        let dialog = match self.operation {
            Operation::Compress => {
                let result = FileDialog::new()
                    .set_title("选择要压缩的文件或目录")
                    .pick_folder();
                
                if result.is_none() {
                    FileDialog::new()
                        .set_title("选择要压缩的文件")
                        .pick_file()
                } else {
                    result
                }
            },
            Operation::Extract | Operation::List => FileDialog::new()
                .set_title("选择ZIP文件")
                .add_filter("ZIP文件", &["zip"])
                .pick_file(),
        };

        if let Some(path) = dialog {
            self.source_path = Some(path.clone());
            
            if self.operation == Operation::Compress && self.output_path.is_none() {
                let mut output = path;
                if output.is_dir() {
                    if let Some(file_name) = output.file_name() {
                        let zip_name = format!("{}.zip", file_name.to_string_lossy());
                        output = output.with_file_name(zip_name);
                    }
                } else {
                    output = output.with_extension("zip");
                }
                self.output_path = Some(output);
            }
        }
    }

    fn select_output_path(&mut self) {
        match self.operation {
            Operation::Compress => {
                if let Some(path) = FileDialog::new()
                    .set_title("选择输出ZIP文件位置")
                    .add_filter("ZIP文件", &["zip"])
                    .save_file() {
                    self.output_path = Some(path);
                }
            }
            Operation::Extract => {
                if let Some(path) = FileDialog::new()
                    .set_title("选择解压目录")
                    .pick_folder() {
                    self.output_path = Some(path);
                }
            }
            _ => {}
        }
    }

    fn execute_operation(&mut self) {
        match self.operation {
            Operation::Compress => {
                if let (Some(source), Some(output)) = (&self.source_path, &self.output_path) {
                    let source = source.clone();
                    let output = output.clone();
                    let level = self.compression_level;
                    
                    let (tx, rx) = mpsc::channel();
                    self.result_receiver = Some(rx);
                    self.operation_state = OperationState::InProgress;
                    
                    thread::spawn(move || {
                        match compress::compress(&source, &output, level) {
                            Ok(_) => {
                                let _ = tx.send(format!("压缩完成: {}", output.display()));
                            }
                            Err(e) => {
                                let _ = tx.send(format!("压缩失败: {}", e));
                            }
                        }
                    });
                } else {
                    self.operation_state = OperationState::Error("请选择源文件/目录和输出文件".to_string());
                }
            }
            Operation::Extract => {
                if let Some(source) = &self.source_path {
                    let source = source.to_string_lossy().to_string();
                    let output = self.output_path.clone();
                    let overwrite = self.overwrite;
                    
                    let (tx, rx) = mpsc::channel();
                    self.result_receiver = Some(rx);
                    self.operation_state = OperationState::InProgress;
                    
                    thread::spawn(move || {
                        match extract::extract(&source, output.as_deref(), overwrite) {
                            Ok(_) => {
                                let _ = tx.send("解压完成".to_string());
                            }
                            Err(e) => {
                                let _ = tx.send(format!("解压失败: {}", e));
                            }
                        }
                    });
                } else {
                    self.operation_state = OperationState::Error("请选择ZIP文件".to_string());
                }
            }
            Operation::List => {
                if let Some(source) = &self.source_path {
                    let source = source.to_string_lossy().to_string();
                    
                    let (tx, rx) = mpsc::channel();
                    self.result_receiver = Some(rx);
                    self.operation_state = OperationState::InProgress;
                    
                    thread::spawn(move || {
                        match list::list_zip_contents(&source) {
                            Ok(content) => {
                                let _ = tx.send(content);
                            }
                            Err(e) => {
                                let _ = tx.send(format!("列表显示失败: {}", e));
                            }
                        }
                    });
                } else {
                    self.operation_state = OperationState::Error("请选择ZIP文件".to_string());
                }
            }
        }
    }
}

pub fn load_icon_data() -> Option<eframe::IconData> {
    const ICON_BYTES: &[u8] = include_bytes!("../assets/p.png");
    
    image::load_from_memory(ICON_BYTES)
        .ok()
        .and_then(|img| {
            let rgba = img.to_rgba8();
            Some(eframe::IconData {
                rgba: rgba.to_vec(),
                width: rgba.width(),
                height: rgba.height(),
            })
        })
}

pub fn run_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(600.0, 400.0)),
        icon_data: load_icon_data(),
        ..Default::default()
    };
    
    eframe::run_native(
        "ZIP工具",
        options,
        Box::new(|_cc| Box::new(ZipToolApp::default())),
    )
} 