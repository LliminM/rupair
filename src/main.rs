use anyhow::{Context, Result};
use rupair::{analyzer, solver::BufferSolver, rectifier};
use std::env;
use std::fs;
use std::path::Path;
use syn::parse_file;
use walkdir::WalkDir;
use z3::Context as Z3Context;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <file_or_directory>", args[0]);
        return Ok(());
    }

    let path = &args[1];
    process_path(path)?;

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
    println!("Processing file: {}", path.display());

    let source = fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let ast = parse_file(&source).with_context(|| format!("Failed to parse {}", path.display()))?;

    let candidates = analyzer::find_buffer_overflows(&ast);
    if candidates.is_empty() {
        println!("No buffer overflow candidates found in {}", path.display());
        return Ok(());
    }

    println!("Found {} potential buffer overflow(s) in {}", candidates.len(), path.display());
    
    // Create Z3 context and solver
    let z3_ctx = Z3Context::new(&z3::Config::new());
    let mut solver = BufferSolver::new(&z3_ctx);
    
    // 存储实际确认的溢出
    let mut confirmed_overflows = Vec::new();
    
    // Process each candidate
    for candidate in &candidates {
        println!("  - {}: {}", candidate.location, candidate.operation);
        
        // Add buffer and check for overflow
        // (在真实场景中，我们需要从代码中提取真实的缓冲区大小)
        let buffer_size = 10; // 这里只是示例，实际应该分析源代码中的缓冲区大小
        solver.add_buffer(&candidate.buffer_name, buffer_size);
        
        // 估计偏移量 (同样，在真实场景中需要从代码中提取)
        let offset = 15; // 示例偏移量
        
        let result = solver.check_overflow(&candidate.buffer_name, offset);
        if result.is_overflow {
            println!("    CONFIRMED OVERFLOW: Buffer size {} with offset {}", 
                     result.buffer_size, result.offset);
            
            // 添加到确认的溢出列表
            confirmed_overflows.push(candidate.clone());
            
            // Generate test case
            let test = solver.generate_test_case(&candidate.buffer_name, offset);
            println!("    Test case:\n{}", test);
        } else {
            println!("    Not a real overflow");
        }
    }
    
    // 如果有确认的溢出，使用rectifier修复代码
    if !confirmed_overflows.is_empty() {
        println!("\nFixing {} confirmed buffer overflow(s)...", confirmed_overflows.len());
        
        // 修复代码
        let fixed_code = rectifier::rectify(&ast, &confirmed_overflows);
        
        // 保存修复后的代码到新文件
        let fixed_path = path.with_file_name(format!(
            "{}_fixed.rs", 
            path.file_stem().unwrap().to_string_lossy()
        ));
        fs::write(&fixed_path, fixed_code)
            .with_context(|| format!("Failed to write fixed code to {}", fixed_path.display()))?;
        
        println!("Fixed code written to {}", fixed_path.display());
    }

    Ok(())
}