#[cfg(feature = "with-rustc")]
extern crate rustc_driver;

use std::path::PathBuf;
use anyhow::Result;
use walkdir::WalkDir;
use regex::Regex;
use syn::{self, parse_file};
use std::fs;

#[derive(Debug)]
pub struct BufferOverflow {
    pub location: String,
    pub operation_type: String,
    pub description: String,
    pub fix_suggestion: String,
}

pub struct MirAnalyzer {
    output_dir: PathBuf,
    source_file: PathBuf,
    vec_allocations: Vec<String>,
    pointer_operations: Vec<String>,
    buffer_overflows: Vec<BufferOverflow>,
}

impl MirAnalyzer {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { 
            output_dir,
            source_file: PathBuf::new(),
            vec_allocations: Vec::new(),
            pointer_operations: Vec::new(),
            buffer_overflows: Vec::new(),
        }
    }

    pub fn set_source_file(&mut self, path: PathBuf) {
        self.source_file = path;
    }

    pub fn analyze(&mut self) -> Result<()> {
        // 分析源代码
        self.analyze_source_code()?;
        
        // 分析MIR文件
        for entry in WalkDir::new(&self.output_dir) {
            let entry = entry?;
            if entry.path().extension().map_or(false, |ext| ext == "mir") {
                let content = fs::read_to_string(entry.path())?;
                self.analyze_mir_content(&content)?;
            }
        }

        // 输出分析结果
        self.print_analysis_results();

        Ok(())
    }

    fn analyze_source_code(&mut self) -> Result<()> {
        let content = fs::read_to_string(&self.source_file)?;
        let ast = parse_file(&content)?;

        // 分析向量分配
        self.find_vec_allocations(&content)?;
        
        // 分析指针操作
        self.find_pointer_operations(&content)?;
        
        // 检测缓冲区溢出
        self.detect_buffer_overflows(&content)?;

        Ok(())
    }

    fn analyze_mir_content(&mut self, content: &str) -> Result<()> {
        // 分析MIR中的向量操作
        self.find_vec_allocations(content)?;
        
        // 分析MIR中的指针操作
        self.find_pointer_operations(content)?;
        
        // 检测MIR中的缓冲区溢出
        self.detect_buffer_overflows(content)?;

        Ok(())
    }

    fn print_analysis_results(&self) {
        if !self.buffer_overflows.is_empty() {
            println!("\n发现潜在的缓冲区溢出问题：");
            println!("=========================");
            
            for (i, overflow) in self.buffer_overflows.iter().enumerate() {
                println!("\n问题 #{}", i + 1);
                println!("位置: {}", overflow.location);
                println!("操作类型: {}", overflow.operation_type);
                println!("描述: {}", overflow.description);
                println!("修复建议: {}", overflow.fix_suggestion);
                println!("--------------------------");
            }
        } else {
            println!("\n未发现缓冲区溢出问题。");
        }
    }

    fn find_vec_allocations(&mut self, content: &str) -> Result<()> {
        // 查找Vec::new()和Vec::with_capacity()调用
        let vec_new_regex = Regex::new(r"Vec::<[^>]*>::new\(\)")?;
        let vec_with_cap_regex = Regex::new(r"Vec::<[^>]*>::with_capacity\([^)]*\)")?;
        
        // 查找push操作
        let vec_push_regex = Regex::new(r"\.push\([^)]*\)")?;

        for line in content.lines() {
            if vec_new_regex.is_match(line) || vec_with_cap_regex.is_match(line) {
                self.vec_allocations.push(line.trim().to_string());
            }
            if vec_push_regex.is_match(line) {
                self.vec_allocations.push(line.trim().to_string());
            }
        }
        Ok(())
    }

    fn find_pointer_operations(&mut self, content: &str) -> Result<()> {
        // 查找指针偏移操作
        let offset_regex = Regex::new(r"(offset|add|sub)\([^)]*\)")?;
        
        // 查找解引用操作
        let deref_regex = Regex::new(r"\*[^*]*")?;
        
        // 查找指针算术运算
        let ptr_arith_regex = Regex::new(r"ptr::(add|sub|offset)")?;

        for line in content.lines() {
            if offset_regex.is_match(line) || deref_regex.is_match(line) || ptr_arith_regex.is_match(line) {
                self.pointer_operations.push(line.trim().to_string());
            }
        }
        Ok(())
    }

    fn detect_buffer_overflows(&mut self, content: &str) -> Result<()> {
        // 检查缓冲区大小和数据大小的比较
        let mut buffer_size: Option<usize> = None;
        let mut data_size: Option<usize> = None;

        // 查找缓冲区大小定义
        let vec_size_regex = Regex::new(r"vec!\[.*?;\s*(\d+)\]")?;
        for line in content.lines() {
            if let Some(cap) = vec_size_regex.captures(line) {
                if let Some(size_str) = cap.get(1) {
                    if let Ok(size) = size_str.as_str().parse::<usize>() {
                        buffer_size = Some(size);
                    }
                }
            }
        }

        // 查找数据大小定义
        let data_vec_regex = Regex::new(r"vec!\[((?:\d+,\s*)*\d+)\]")?;
        for line in content.lines() {
            if let Some(cap) = data_vec_regex.captures(line) {
                if let Some(data_str) = cap.get(1) {
                    let count = data_str.as_str().split(',').count();
                    data_size = Some(count);
                }
            }
        }

        // 检查循环中的指针操作
        let loop_regex = Regex::new(r"for\s*\([^)]*\)\s*\{[^}]*\}")?;
        let ptr_op_regex = Regex::new(r"\*ptr\.add\(([^)]+)\)")?;

        for line in content.lines() {
            // 检查不安全的指针操作
            if line.contains("unsafe") {
                self.buffer_overflows.push(BufferOverflow {
                    location: line.trim().to_string(),
                    operation_type: "Unsafe Block".to_string(),
                    description: "发现不安全代码块，需要仔细检查指针操作".to_string(),
                    fix_suggestion: "添加显式的边界检查，确保指针操作安全".to_string(),
                });
            }

            // 检查循环中的指针操作
            if line.contains("for") && line.contains("enumerate") {
                if let (Some(buf_size), Some(dat_size)) = (buffer_size, data_size) {
                    if dat_size > buf_size {
                        self.buffer_overflows.push(BufferOverflow {
                            location: line.trim().to_string(),
                            operation_type: "Buffer Overflow".to_string(),
                            description: format!(
                                "检测到潜在的缓冲区溢出：目标缓冲区大小为 {}，但尝试写入 {} 个元素",
                                buf_size, dat_size
                            ),
                            fix_suggestion: format!(
                                "1. 增加缓冲区大小到至少 {} 个元素\n   2. 或添加边界检查: if i < buffer.len() {{ ... }}",
                                dat_size
                            ),
                        });
                    }
                }
            }

            // 检查指针算术运算
            if let Some(caps) = ptr_op_regex.captures(line) {
                if let Some(index_expr) = caps.get(1) {
                    self.buffer_overflows.push(BufferOverflow {
                        location: line.trim().to_string(),
                        operation_type: "Pointer Arithmetic".to_string(),
                        description: format!(
                            "检测到未检查的指针偏移操作: {}",
                            index_expr.as_str()
                        ),
                        fix_suggestion: "在进行指针操作前添加显式的边界检查：if i < buffer.len() { ... }".to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}