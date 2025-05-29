#![feature(rustc_private)]

use anyhow::Result;
use std::path::PathBuf;
use std::env;
use rupair::RuPair;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <source_file>", args[0]);
        return Ok(());
    }

    let source_file = PathBuf::from(&args[1]);
    let output_dir = PathBuf::from("output");

    let rupair = RuPair::new(source_file.clone(), output_dir.clone());
    let (fixed_code, report) = rupair.analyze_and_fix()?;

    let fixed_file = output_dir.join("fixed.rs");
    let report_file = output_dir.join("report.md");
    
    std::fs::write(&fixed_file, fixed_code)?;
    std::fs::write(&report_file, report)?;

    println!("\n分析完成！");
    println!("- 修复代码: {}", fixed_file.display());
    println!("- 报告: {}", report_file.display());

    Ok(())
}