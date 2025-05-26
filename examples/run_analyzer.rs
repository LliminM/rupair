use rupair::mir_analyzer::MirAnalyzer;
use std::path::PathBuf;
use std::fs;

fn main() {
    // 创建分析器实例
    let mut analyzer = MirAnalyzer::new(PathBuf::from("target/debug"));
    
    // 设置要分析的源文件
    let source_file = PathBuf::from("examples/buffer_overflow_test.rs");
    analyzer.set_source_file(source_file.clone());
    
    // 运行分析
    if let Err(e) = analyzer.analyze() {
        eprintln!("分析过程中出错: {}", e);
        return;
    }

    // 获取所有的问题
    let fixes = analyzer.get_fixes();
    
    // 生成错误报告
    let mut report = String::from("# 缓冲区溢出安全分析报告\n\n");
    report.push_str("## 分析概述\n\n");
    report.push_str(&format!("- 分析文件: {}\n", source_file.display()));
    report.push_str(&format!("- 发现问题数量: {}\n\n", fixes.len()));
    
    // 生成完整的修复代码
    let mut complete_fixed_code = String::new();
    
    for (i, fix) in fixes.iter().enumerate() {
        // 添加错误报告
        let error_report = analyzer.get_rectifier().generate_error_report(fix);
        report.push_str(&format!("## 问题 #{}\n\n", i + 1));
        report.push_str(&format!("### 基本信息\n"));
        report.push_str(&format!("- 问题类型: {}\n", error_report.issue_type));
        report.push_str(&format!("- 位置: {}\n", error_report.location));
        report.push_str(&format!("- 风险等级: {}\n", error_report.risk_level));
        report.push_str(&format!("\n### 详细说明\n"));
        report.push_str(&format!("- 描述: {}\n", error_report.description));
        report.push_str(&format!("- 潜在影响: {}\n", error_report.impact));
        report.push_str(&format!("- 修复建议: {}\n\n", error_report.recommendation));
        
        // 获取完整的修复代码
        if let Ok(fixed_code) = analyzer.get_rectifier().generate_complete_fix(fix) {
            complete_fixed_code.push_str(&format!("// 修复问题 #{}\n", i + 1));
            complete_fixed_code.push_str(&fixed_code);
            complete_fixed_code.push_str("\n");
        }
    }
    
    // 保存错误报告
    let report_file = source_file.with_file_name("buffer_overflow_report.md");
    fs::write(&report_file, report).unwrap();
    
    // 保存完整的修复代码
    let fixed_file = source_file.with_file_name("buffer_overflow_fixed.rs");
    fs::write(&fixed_file, complete_fixed_code).unwrap();
    
    println!("\n分析完成！");
    println!("- 安全分析报告已保存到: {}", report_file.display());
    println!("- 修复后的代码已保存到: {}", fixed_file.display());
} 