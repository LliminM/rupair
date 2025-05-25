#![feature(rustc_private)]

use anyhow::Result;
use rupair::MirAnalyzer;  // 直接从 rupair 导入 MirAnalyzer
use std::path::Path;
use walkdir::WalkDir;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    // 检查命令行参数
    if args.len() < 2 {
        println!("Usage: {} <path_to_rust_file_or_directory>", args[0]);
        return Ok(());
    }

    // 处理输入路径
    process_path(&args[1])?;

    Ok(())
}

fn process_path(path: &str) -> Result<()> {
    let path = Path::new(path);
    if path.is_dir() {
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
                process_file(path)?;
            }
        }
    } else if path.is_file() {
        process_file(path)?;
    }
    Ok(())
}

fn process_file(path: &Path) -> Result<()> {
    // 创建MIR输出目录
    let output_dir = path.parent().unwrap().join("mir_output");
    std::fs::create_dir_all(&output_dir)?;

    // 使用rustc生成MIR
    let args = vec![
        String::from("rustc"),
        path.to_string_lossy().into_owned(),
        String::from("--emit=mir"),
        format!("--out-dir={}", output_dir.display()),
    ];

    // 运行rustc生成MIR文件
    std::process::Command::new("rustc")
        .args(&args)
        .output()?;

    // 使用MirAnalyzer分析生成的MIR文件
    let mut analyzer = MirAnalyzer::new(output_dir);
    analyzer.analyze()?;

    Ok(())
}