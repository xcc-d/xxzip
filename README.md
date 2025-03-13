# Zip Tool - 高效的Rust解压缩工具

## 克隆项目

```bash
git clone https://github.com/xcc-d/xxzip.git
cd xxzip
```

这是一个用Rust编写的高性能ZIP文件压缩和解压缩工具，支持文件和目录的压缩、解压缩以及查看ZIP文件内容。提供命令行界面和图形用户界面两种使用方式。

## 功能特点

- 压缩文件和目录到ZIP文件
- 解压缩ZIP文件到指定目录
- 列出ZIP文件内容
- 支持自定义压缩级别
- 显示进度条和操作时间
- 支持覆盖选项
- 提供简洁的GUI界面
- 支持夜间模式
- 动态缓冲区调整（64KB-8MB）
- 智能内存管理优化
- 并行I/O流水线处理
- 大文件内存映射加速

## 安装

确保您已安装Rust和Cargo，然后运行：

```bash

# 编译（不含GUI）
cargo build --release --package zip_tool@0.2.0

# 编译（含GUI）
cargo build --release --features gui --package zip_tool@0.2.0
```

编译后的可执行文件将位于`target/release/zip_tool.exe`。

## 使用方法

### 图形界面

运行`zip_tool.exe`（不带参数）或`zip_tool.exe --gui`启动图形界面。GUI界面提供以下功能：

- 简洁直观的操作界面
- 支持文件和目录选择
- 实时显示操作进度和结果
- 夜间模式支持（点击右上角的模式切换按钮）

![GUI界面截图](gui_screenshot.png)

### 命令行界面

#### 压缩文件或目录

```bash
# 基本用法
zip_tool compress <源文件或目录>

# 指定输出文件
zip_tool compress <源文件或目录> --output <输出文件路径>

# 指定压缩级别 (0-9)
zip_tool compress <源文件或目录> --level 9
```

#### 解压缩ZIP文件

```bash
# 基本用法
zip_tool extract <ZIP文件>

# 指定输出目录
zip_tool extract <ZIP文件> --output-dir <输出目录>

# 覆盖已存在的文件
zip_tool extract <ZIP文件> --overwrite
```

#### 列出ZIP文件内容

```bash
zip_tool list <ZIP文件>
```

## 示例

```bash
# 压缩目录
zip_tool compress my_documents

# 压缩单个文件并指定输出路径
zip_tool compress important.txt --output backup.zip

# 使用最高压缩级别
zip_tool compress large_folder --level 9

# 解压到指定目录
zip_tool extract archive.zip --output-dir extracted_files

# 解压并覆盖已存在的文件
zip_tool extract archive.zip --overwrite

# 查看ZIP文件内容
zip_tool list archive.zip

# 启动GUI模式
zip_tool --gui
```

## 版本历史

### v0.2 (2025-03-14)
- 引入动态缓冲区调整机制
- 优化内存使用策略，降低30%内存占用
- 改进并行流水线处理效率
- 增加大文件内存映射支持

### v0.1 (2025-03-10)
- 基础压缩/解压功能实现
- 基本GUI界面
- 多线程支持

## 许可证

MIT
