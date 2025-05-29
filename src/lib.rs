#![feature(rustc_private)]
#![feature(box_patterns)]
#![feature(proc_macro_diagnostic)]
#![feature(proc_macro_span)]
#![feature(proc_macro_quote)]

#[cfg(feature = "with-rustc")]
extern crate rustc_driver;

pub mod frontend;
pub mod analyzer;
pub mod solver;
pub mod rectifier;
pub mod validator;
pub mod mir_analyzer;

pub use analyzer::OverflowCandidate;
pub use rectifier::{CodeFix, Rectifier, FixType, ErrorReport};
pub use solver::{BufferSolver, BufferConstraint};
pub use validator::*;
pub use mir_analyzer::MirAnalyzer;

use std::path::PathBuf;
use anyhow::Result;
use std::fs;
use regex::Regex;
pub struct RuPair {
    source_file: PathBuf,
    output_dir: PathBuf,
}

impl RuPair {
    pub fn new(source_file: PathBuf, output_dir: PathBuf) -> Self {
        Self { source_file, output_dir }
    }

    pub fn analyze_and_fix(&self) -> Result<(String, String)> {
        let content = fs::read_to_string(&self.source_file)?;
        
        let mut analyzer = MirAnalyzer::new(self.output_dir.clone());
        analyzer.set_source_file(self.source_file.clone());
        analyzer.analyze()?;
    
        let candidates = analyzer.get_fixes();
        let rectifier = Rectifier::new(self.source_file.clone());
        let solver = analyzer.get_solver();
        
        let mut fixed = content.clone();
        let mut fixes = Vec::new();
        
        for candidate in &candidates {
            let constraint = solver.check_overflow(candidate);
            if constraint.is_overflow {
                let fix = rectifier.generate_fix(candidate, &constraint)?;
                fixes.push(fix);
            }
        }
    
        // 调试：打印 fixes
        println!("Generated {} fixes", fixes.len());
        for fix in &fixes {
            println!("Fix: {:?}", fix);
        }
    
        // 替换修复代码
        for fix in &fixes {
            let re = Regex::new(r"(?s)unsafe\s*\{[^{}]*\*ptr\.add\s*\(\d+\)[^{}]*\}").unwrap();
            if re.is_match(&fixed) {
                fixed = re.replace(&fixed, &fix.fixed_code).to_string();
            } else {
                println!("Warning: Could not find unsafe block for fix: {:?}", fix);
                // 后备替换：基于行号
                let lines: Vec<&str> = fixed.lines().collect();
                if fix.location.contains("Line") {
                    if let Ok(line_num) = fix.location.replace("Line ", "").parse::<usize>() {
                        if line_num > 0 && line_num <= lines.len() {
                            let mut new_lines = lines.to_vec();
                            new_lines[line_num - 1] = &fix.fixed_code;
                            fixed = new_lines.join("\n");
                        }
                    }
                }
            }
        }
    
        let mut report = String::from("# Buffer Overflow Analysis Report\n\n");
        report.push_str("## Analysis Overview\n\n");
        report.push_str(&format!("- Source File: {}\n", self.source_file.display()));
        report.push_str(&format!("- Issues Found: {}\n\n", candidates.len()));
        
        for (i, candidate) in candidates.iter().enumerate() {
            let error_report = rectifier.generate_error_report(candidate);
            report.push_str(&format!("## Issue #{}\n\n", i + 1));
            report.push_str(&format!("### Location\n{}\n\n", error_report.location));
            report.push_str(&format!("### Operation Type\n{}\n\n", error_report.issue_type));
            report.push_str(&format!("### Description\n{}\n\n", error_report.description));
            report.push_str(&format!("### Fix Suggestion\n{}\n\n", error_report.recommendation));
            
            if let Some(fix) = fixes.iter().find(|f| f.location == error_report.location) {
                report.push_str("### Original Code\n```rust\n");
                report.push_str(&fix.original_code);
                report.push_str("\n```\n\n");
                report.push_str("### Fixed Code\n```rust\n");
                report.push_str(&fix.fixed_code);
                report.push_str("\n```\n\n");
            }
        }
    
        Ok((fixed, report))
    }
}