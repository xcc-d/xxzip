fn main() {
    #[cfg(target_os = "windows")]
    {
        // 只在Windows上执行
        println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
        println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
    }
} 