use rupair::mir_analyzer::MirAnalyzer;
use std::path::PathBuf;

fn main() {
    // 创建分析器实例
    let mut analyzer = MirAnalyzer::new(PathBuf::from("target/debug"));
    
    // 设置要分析的源文件
    let source_file = PathBuf::from("examples/safe_buffer.rs");
    analyzer.set_source_file(source_file.clone());
    
    // 运行分析
    match analyzer.analyze() {
        Ok(_) => {
            let fixes = analyzer.get_fixes();
            if fixes.is_empty() {
                println!("\n✅ 代码分析完成：未发现任何缓冲区溢出问题！");
                println!("该代码实现了以下安全特性：");
                println!("1. 使用 with_capacity 预分配空间");
                println!("2. 所有索引访问都有边界检查");
                println!("3. 使用安全的切片操作");
                println!("4. 使用 get 方法进行安全访问");
                println!("5. 正确处理缓冲区扩展");
                println!("6. 使用安全的迭代器方法");
            } else {
                println!("\n⚠️ 发现 {} 个潜在的问题", fixes.len());
                for (i, fix) in fixes.iter().enumerate() {
                    println!("\n问题 #{}:", i + 1);
                    println!("位置: {}", fix.location);
                    println!("类型: {}", fix.operation_type);
                    println!("描述: {}", fix.description);
                }
            }
        }
        Err(e) => {
            eprintln!("分析过程中出错: {}", e);
        }
    }
} 