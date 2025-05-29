use std::path::PathBuf;
use anyhow::Result;
use syn::{self, parse_file,spanned::Spanned};
use std::fs;
use quote::ToTokens;
use proc_macro2::{Span, LineColumn};

use crate::analyzer::OverflowCandidate;

#[derive(Debug)]
pub struct AnalysisResult {
    pub unsafe_blocks: Vec<String>,
    pub allocations: Vec<String>,
    pub overflow_candidates: Vec<OverflowCandidate>,
}

pub struct Frontend {
    source_file: PathBuf,
}

impl Frontend {
    pub fn new() -> Self {
        Self {
            source_file: PathBuf::new(),
        }
    }

    pub fn set_source_file(&mut self, path: PathBuf) {
        self.source_file = path.clone();
    }

    pub fn analyze(&mut self) -> Result<AnalysisResult> {
        let content = fs::read_to_string(&self.source_file)?;
        let _ast = parse_file(&content)?;

        let mut result = AnalysisResult {
            unsafe_blocks: Vec::new(),
            allocations: Vec::new(),
            overflow_candidates: Vec::new(),
        };

        self.analyze_ast(&content, &mut result)?;
        Ok(result)
    }

    fn analyze_ast(&self, content: &str, result: &mut AnalysisResult) -> Result<()> {
        let ast = parse_file(content)?;
        let mut visitor = AstVisitor::new();
        visitor.visit_file(&ast);

        result.unsafe_blocks = visitor.unsafe_blocks;
        result.allocations = visitor.allocations;
        result.overflow_candidates = visitor.overflow_candidates;
        Ok(())
    }
}

struct AstVisitor {
    unsafe_blocks: Vec<String>,
    allocations: Vec<String>,
    overflow_candidates: Vec<OverflowCandidate>,
}

impl AstVisitor {
    fn new() -> Self {
        Self {
            unsafe_blocks: Vec::new(),
            allocations: Vec::new(),
            overflow_candidates: Vec::new(),
        }
    }

    fn visit_file(&mut self, file: &syn::File) {
        for item in &file.items {
            self.visit_item(item);
        }
    }

    fn visit_item(&mut self, item: &syn::Item) {
        match item {
            syn::Item::Fn(func) => {
                for stmt in &func.block.stmts {
                    self.visit_stmt(stmt);
                }
            }
            _ => {}
        }
    }

    fn visit_stmt(&mut self, stmt: &syn::Stmt) {
        match stmt {
            syn::Stmt::Item(item) => self.visit_item(item),
            syn::Stmt::Expr(expr, ..) => self.visit_expr(expr),
            syn::Stmt::Local(syn::Local { init: Some(init), .. }) => {
                self.visit_expr(&init.expr);
            }
            syn::Stmt::Local(_) => {},
            stmt @ syn::Stmt::Macro(..) => {
                if let syn::Stmt::Macro(macro_stmt) = stmt {
                    if let Some(ident) = macro_stmt.mac.path.get_ident() {
                        if ident == "vec" {
                            let span = macro_stmt.span();
                            let start = span.start();
                            println!("Vec macro at line {}, column {}", start.line, start.column); // 调试
                            self.allocations.push(macro_stmt.to_token_stream().to_string());
                        }
                    }
                }
            }
        }
    }

    fn visit_expr(&mut self, expr: &syn::Expr) {
        match expr {
            syn::Expr::Unsafe(expr) => {
                let span = expr.span();
                let start = span.start();
                println!("Unsafe block at line {}, column {}", start.line, start.column); // 调试
                self.unsafe_blocks.push(expr.to_token_stream().to_string());
                for stmt in &expr.block.stmts {
                    if let syn::Stmt::Expr(syn::Expr::MethodCall(method_call), _) = stmt {
                        if method_call.method == "add" {
                            let span = method_call.span();
                            let start = span.start();
                            println!("ptr.add at line {}, column {}", start.line, start.column); // 调试
                            self.overflow_candidates.push(OverflowCandidate {
                                location: method_call.to_token_stream().to_string(),
                                buffer_name: "buffer".to_string(),
                                operation: "pointer_offset".to_string(),
                                line: start.line,
                                column: start.column,
                                buffer_size: Some(10),
                                offset: Some(15),
                            });
                        }
                    }
                }
            },
            syn::Expr::Macro(expr) => {
                if let Some(ident) = expr.mac.path.get_ident() {
                    if ident == "vec" {
                        let span = expr.span();
                        let start = span.start();
                        println!("Vec macro at line {}, column {}", start.line, start.column); // 调试
                        self.allocations.push(expr.to_token_stream().to_string());
                        self.overflow_candidates.push(OverflowCandidate {
                            location: expr.to_token_stream().to_string(),
                            buffer_name: "vec".to_string(),
                            operation: "allocation".to_string(),
                            line: start.line,
                            column: start.column,
                            buffer_size: None,
                            offset: None,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ast() {
        let mut frontend = Frontend::new();
        frontend.set_source_file(PathBuf::from("examples/test.rs"));
        
        let result = frontend.analyze().unwrap();
        assert!(!result.unsafe_blocks.is_empty());
    }
}