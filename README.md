# Zip Tool - 高效的Rust解压缩工具(zbd)

## 克隆项目

```bash
git clone https://github.com/xcc-d/xxzip.git
cd xxzip
```

这是一个用Rust编写的高性能ZIP文件压缩和解压缩工具，支持文件和目录的压缩、解压缩以及查看ZIP文件内容。提供纯图形用户界面，操作简单直观。

## 功能特点

- 压缩文件和目录到ZIP文件
- 解压缩ZIP文件到指定目录
- 列出ZIP文件内容
- 支持自定义压缩级别
- 显示进度条和操作时间
- 支持覆盖选项
- 提供简洁的GUI界面
- 支持夜间模式
- 动态缓冲区调整（64KB-2MB）
- 智能内存管理优化
- 并行I/O流水线处理
- 大文件内存映射加速

## 安装

确保您已安装Rust和Cargo，然后运行：

```bash
# 编译
cargo build --release
```

编译后的可执行文件将位于`target/release/zip_tool.exe`。

## 使用方法

### 图形界面

双击`zip_tool.exe`启动图形界面。GUI界面提供以下功能：

- 简洁直观的操作界面
- 支持文件和目录选择
- 实时显示操作进度和结果
- 夜间模式支持（点击右上角的模式切换按钮）

![GUI界面截图](gui_screenshot.png)

## 版本历史

### v0.3.1 (2025-03-25)
- 移除命令行界面，纯GUI应用程序
- 优化Windows应用程序体验，去除控制台窗口
- 添加完整的日志系统，替代控制台输出

### v0.3.0 (2025-03-20)
- 优化内存使用，添加缓冲区大小上限(2MB)，避免过度消耗内存
- 降低内存映射阈值(从1GB到100MB)，更合理地使用内存映射
- 添加内存映射失败时的回退机制，提高稳定性
- 改进错误处理，替换所有unwrap_or_default()调用
- 改进GUI文件选择对话框，支持同时选择文件和文件夹
- 优化列表功能，添加更多信息和格式化输出
- 修复所有编译警告

### v0.2.0 (2025-03-14)
- 引入动态缓冲区调整机制
- 优化内存使用策略，降低30%内存占用
- 改进并行流水线处理效率
- 增加大文件内存映射支持

### v0.1.0 (2025-03-10)
- 基础压缩/解压功能实现
- 基本GUI界面
- 多线程支持

## 许可证

MIT