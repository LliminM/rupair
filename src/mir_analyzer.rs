#[cfg(feature = "with-rustc")]
extern crate rustc_driver;

use std::path::PathBuf;
use anyhow::Result;
use walkdir::WalkDir;
use regex::Regex;
use syn::{self, parse_file};
use std::fs;

use crate::analyzer::OverflowCandidate;
use crate::rectifier::Rectifier;
use crate::solver::BufferSolver;

pub struct MirAnalyzer {
    output_dir: PathBuf,
    source_file: PathBuf,
    vec_allocations: Vec<String>,
    pointer_operations: Vec<String>,
    overflow_candidates: Vec<OverflowCandidate>,
    rectifier: Option<Rectifier>,
    solver: Option<&'static mut BufferSolver<'static>>,
}

impl MirAnalyzer {
    pub fn new(output_dir: PathBuf) -> Self {
        let ctx = Box::leak(Box::new(z3::Context::new(&z3::Config::new())));
        let solver = Box::leak(Box::new(BufferSolver::new(ctx)));
        
        Self { 
            output_dir,
            source_file: PathBuf::new(),
            vec_allocations: Vec::new(),
            pointer_operations: Vec::new(),
            overflow_candidates: Vec::new(),
            rectifier: None,
            solver: Some(solver),
        }
    }

    pub fn set_source_file(&mut self, path: PathBuf) {
        self.source_file = path.clone();
        self.rectifier = Some(Rectifier::new(path));
    }

    pub fn analyze(&mut self) -> Result<()> {
        self.analyze_source_code()?;
        
        for entry in WalkDir::new(&self.output_dir) {
            let entry = entry?;
            if entry.path().extension().map_or(false, |ext| ext == "mir") {
                let content = fs::read_to_string(entry.path())?;
                self.analyze_mir_content(&content)?;
            }
        }

        self.print_analysis_results();
        Ok(())
    }

    fn analyze_source_code(&mut self) -> Result<()> {
        let content = fs::read_to_string(&self.source_file)?;
        let _ast = parse_file(&content)?;
        self.find_vec_allocations(&content)?;
        self.find_pointer_operations(&content)?;
        Ok(())
    }

    fn analyze_mir_content(&mut self, content: &str) -> Result<()> {
        self.find_vec_allocations(content)?;
        self.find_pointer_operations(content)?;
        self.detect_buffer_overflows(content)?;
        Ok(())
    }

    fn print_analysis_results(&self) {
        if !self.overflow_candidates.is_empty() {
            println!("\n发现潜在的缓冲区溢出问题：");
            println!("=========================");
            
            for (i, candidate) in self.overflow_candidates.iter().enumerate() {
                println!("\n问题 #{}", i + 1);
                println!("位置: {}", candidate.location);
                println!("操作类型: {}", candidate.operation);
                println!("描述: 未检查的指针偏移操作: {:?}", candidate.offset);
                println!("--------------------------");
            }
        } else {
            println!("\n未发现缓冲区溢出问题。");
        }
    }

    fn find_vec_allocations(&mut self, content: &str) -> Result<()> {
        let vec_regex = Regex::new(r"vec!\[[^\]]*\]")?;
        
        for line in content.lines() {
            if let Some(caps) = vec_regex.captures(line) {
                let vec_expr = caps.get(0).unwrap().as_str();
                println!("Found vec! for buffer with size {:?}", None::<usize>);
                self.vec_allocations.push(vec_expr.to_string());
            }
        }
        
        Ok(())
    }

    fn find_pointer_operations(&mut self, content: &str) -> Result<()> {
        let ptr_regex = Regex::new(r"ptr\.add\((\d+)\)")?;
        
        for (i, line) in content.lines().enumerate() {
            if line.contains("unsafe") {
                println!("Found unsafe block");
            }
            
            if let Some(caps) = ptr_regex.captures(line) {
                if let Some(offset) = caps.get(1) {
                    println!("Found add for pointer ptr with offset Some({})", offset.as_str());
                    self.pointer_operations.push(line.trim().to_string());
                    let offset_value = offset.as_str().parse::<usize>().unwrap_or(0);
                    self.overflow_candidates.push(OverflowCandidate {
                        location: line.trim().to_string(),
                        buffer_name: "buffer".to_string(),
                        operation: "pointer_offset".to_string(),
                        line: i + 1,
                        column: 0,
                        buffer_size: None,
                        offset: Some(offset_value),
                    });
                }
            }
        }
        
        Ok(())
    }

    fn detect_buffer_overflows(&mut self, content: &str) -> Result<()> {
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i].trim();
            
            if line.contains("unsafe") {
                let mut block_start = i;
                let mut block_end = i;
                let mut brace_count = 0;
                
                while block_start > 0 && !lines[block_start].contains("unsafe") {
                    block_start -= 1;
                }
                
                while block_end < lines.len() {
                    let current_line = lines[block_end].trim();
                    if current_line.contains("{") {
                        brace_count += 1;
                    }
                    if current_line.contains("}") {
                        brace_count -= 1;
                        if brace_count == 0 {
                            break;
                        }
                    }
                    block_end += 1;
                }
                
                let unsafe_block = lines[block_start..=block_end].join("\n");
                
                let offset_regex = Regex::new(r"\*ptr\.add\((\d+)\)")?;
                if let Some(caps) = offset_regex.captures(&unsafe_block) {
                    if let Some(offset) = caps.get(1) {
                        let offset_value = offset.as_str().parse::<usize>().unwrap_or(0);
                        self.overflow_candidates.push(OverflowCandidate {
                            location: unsafe_block.trim().to_string(),
                            buffer_name: "buffer".to_string(),
                            operation: "pointer_offset".to_string(),
                            line: block_start + 1,
                            column: 0,
                            buffer_size: None,
                            offset: Some(offset_value),
                        });
                    }
                }
                
                i = block_end + 1;
            } else {
                i += 1;
            }
        }

        Ok(())
    }

    pub fn get_fixes(&self) -> Vec<OverflowCandidate> {
        self.overflow_candidates.clone()
    }

    pub fn get_rectifier(&self) -> &Rectifier {
        self.rectifier.as_ref().expect("Rectifier not initialized")
    }

    pub fn get_solver(&mut self) -> &mut BufferSolver<'static> {
        self.solver.as_mut().expect("Solver not initialized")
    }
}