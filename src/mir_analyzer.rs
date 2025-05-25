#[cfg(feature = "with-rustc")]
extern crate rustc_driver;

use std::path::PathBuf;
use anyhow::Result;
use walkdir::WalkDir;
use regex::Regex;

#[derive(Debug)]
pub struct BufferOverflow {
    location: String,
    operation_type: String,
    description: String,
}

pub struct MirAnalyzer {
    output_dir: PathBuf,
    vec_allocations: Vec<String>,
    pointer_operations: Vec<String>,
    buffer_overflows: Vec<BufferOverflow>,
}

impl MirAnalyzer {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { 
            output_dir,
            vec_allocations: Vec::new(),
            pointer_operations: Vec::new(),
            buffer_overflows: Vec::new(),
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
        let deref_regex = Regex::new(r"(*[^*]*)")?;
        
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
        // 检查每个Vec操作
        for vec_op in &self.vec_allocations {
            // 检查是否有边界检查
            if !content.contains("assert") && !content.contains("bounds_check") {
                // 检查是否有潜在的越界访问
                if vec_op.contains("index") || vec_op.contains("get_unchecked") {
                    self.buffer_overflows.push(BufferOverflow {
                        location: vec_op.to_string(),
                        operation_type: "Vec access".to_string(),
                        description: "Potential out of bounds access".to_string(),
                    });
                }
            }
        }

        // 检查指针操作
        for ptr_op in &self.pointer_operations {
            // 检查是否有不安全的指针操作
            if ptr_op.contains("unsafe") || ptr_op.contains("get_unchecked") {
                self.buffer_overflows.push(BufferOverflow {
                    location: ptr_op.to_string(),
                    operation_type: "Pointer operation".to_string(),
                    description: "Unsafe pointer operation detected".to_string(),
                });
            }
        }

        // 检查特定的危险模式
        let dangerous_patterns = [
            (r"ptr::offset\([^)]*\)", "Pointer offset without bounds check"),
            (r"get_unchecked\([^)]*\)", "Unchecked array access"),
            (r"as \*mut", "Raw pointer cast"),
            (r"transmute", "Memory transmutation"),
        ];

        for (pattern, desc) in dangerous_patterns.iter() {
            let regex = Regex::new(pattern)?;
            for line in content.lines() {
                if regex.is_match(line) {
                    self.buffer_overflows.push(BufferOverflow {
                        location: line.trim().to_string(),
                        operation_type: "Dangerous operation".to_string(),
                        description: desc.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    pub fn analyze(&mut self) -> Result<()> {
        for entry in WalkDir::new(&self.output_dir) {
            let entry = entry?;
            if entry.path().extension().map_or(false, |ext| ext == "mir") {
                let content = std::fs::read_to_string(entry.path())?;
                
                // 执行所有分析
                self.find_vec_allocations(&content)?;
                self.find_pointer_operations(&content)?;
                self.detect_buffer_overflows(&content)?;

                // 打印分析结果
                if !self.buffer_overflows.is_empty() {
                    println!("Found potential buffer overflows in {}:", entry.path().display());
                    for overflow in &self.buffer_overflows {
                        println!("  - Location: {}", overflow.location);
                        println!("    Type: {}", overflow.operation_type);
                        println!("    Description: {}", overflow.description);
                        println!();
                    }
                }
            }
        }
        Ok(())
    }
}