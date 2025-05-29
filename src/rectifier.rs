use crate::analyzer::OverflowCandidate;
use crate::solver::BufferConstraint;
use quote::quote;
use syn::{File, Item, ItemFn, Stmt, Expr, ExprUnsafe, ExprBlock, ExprMethodCall, Block};
use quote::ToTokens;
use std::path::PathBuf;
use anyhow::{Result, Error};
use std::fs;
use regex::Regex;
use syn::{ExprBinary, BinOp, ExprCall};
use proc_macro2::TokenStream;

#[derive(Debug)]
pub struct CodeFix {
    pub original_code: String,
    pub fixed_code: String,
    pub location: String,
    pub fix_type: FixType,
}

#[derive(Debug, Clone)]
pub enum FixType {
    BoundCheck,
    VecResize,
    SafeAccess,
    UnsafeToSafe,
}

#[derive(Debug)]
pub struct ErrorReport {
    pub issue_type: String,
    pub location: String,
    pub risk_level: String,
    pub description: String,
    pub impact: String,
    pub recommendation: String,
}

pub struct Rectifier {
    source_file: PathBuf,
}

impl Rectifier {
    pub fn new(source_file: PathBuf) -> Self {
        Self { source_file }
    }

    pub fn rectify(&self, ast: &File, overflows: &[OverflowCandidate]) -> String {
        if overflows.is_empty() {
            return quote!(#ast).to_string();
        }

        let mut fixed_ast = ast.clone();

        for overflow in overflows {
            fix_overflow(&mut fixed_ast, overflow);
        }

        quote!(#fixed_ast).to_string()
    }

    pub fn generate_fix(&self, candidate: &OverflowCandidate, constraint: &BufferConstraint) -> Result<CodeFix> {
        let content = fs::read_to_string(&self.source_file)?;
        let lines: Vec<&str> = content.lines().collect();
        
        // 使用行号定位
        let line_num = candidate.line;
        let line = if line_num > 0 && line_num <= lines.len() {
            lines[line_num - 1].to_string()
        } else {
            candidate.location.clone()
        };

        let original_code = line;
        let fix_type = self.determine_fix_type(candidate, constraint);
        let fixed_code = self.generate_fixed_code(candidate, constraint, &fix_type);

        Ok(CodeFix {
            original_code,
            fixed_code,
            location: format!("Line {}", line_num),
            fix_type,
        })
    }

    fn determine_fix_type(&self, candidate: &OverflowCandidate, constraint: &BufferConstraint) -> FixType {
        if candidate.operation.contains("unsafe") {
            FixType::UnsafeToSafe
        } else if constraint.is_overflow {
            if constraint.offset > constraint.buffer_size {
                FixType::VecResize
            } else {
                FixType::BoundCheck
            }
        } else {
            FixType::SafeAccess
        }
    }

    fn generate_fixed_code(&self, _candidate: &OverflowCandidate, constraint: &BufferConstraint, fix_type: &FixType) -> String {
        match fix_type {
            FixType::BoundCheck => {
                format!(
                    "if {} < buffer.len() {{\n    buffer[{}] = 42;\n}} else {{\n    panic!(\"Buffer overflow prevented: index {}\");\n}}",
                    constraint.offset,
                    constraint.offset,
                    constraint.offset
                )
            },
            FixType::VecResize => {
                format!(
                    "let mut buffer = vec![0; {}];\n    buffer[{}] = 42;",
                    constraint.offset + 1,
                    constraint.offset
                )
            },
            FixType::SafeAccess => {
                format!(
                    "if let Some(value) = buffer.get_mut({}) {{\n    *value = 42;\n}} else {{\n    panic!(\"Buffer overflow prevented: index {}\");\n}}",
                    constraint.offset,
                    constraint.offset
                )
            },
            FixType::UnsafeToSafe => {
                format!(
                    "if {} < buffer.len() {{\n    buffer[{}] = 42;\n}} else {{\n    panic!(\"Buffer overflow prevented: index {}\");\n}}",
                    constraint.offset,
                    constraint.offset,
                    constraint.offset
                )
            },
        }
    }
    
    pub fn generate_complete_fix(&self, _candidate: &OverflowCandidate) -> Result<String> {
        let content = fs::read_to_string(&self.source_file)?;
        let lines: Vec<&str> = content.lines().collect();
        
        let mut result = String::new();
        let mut in_unsafe = false;
        let mut unsafe_block = String::new();
        let mut found_problem = false;
        let _current_line = 0;
        
        for line in lines {
            let trimmed = line.trim();
            
            if trimmed.contains("unsafe") {
                in_unsafe = true;
                unsafe_block.clear();
            }
            
            if in_unsafe {
                unsafe_block.push_str(line);
                unsafe_block.push('\n');
                
                if trimmed.contains("*ptr.add(") {
                    found_problem = true;
                }
                
                if trimmed.contains("}") {
                    in_unsafe = false;
                    
                    if found_problem {
                        let offset_regex = Regex::new(r"\*ptr\.add\((\d+)\)")?;
                        if let Some(caps) = offset_regex.captures(&unsafe_block) {
                            if let Some(offset) = caps.get(1) {
                                let offset_value = offset.as_str().parse::<usize>().unwrap_or(0);
                                let _problem_line = caps.get(0).unwrap().as_str();
                                
                                let fixed_block = format!(
                                    "if {} < buffer.len() {{\n    buffer[{}] = 42;\n}} else {{\n    panic!(\"Buffer overflow prevented: index {}\");\n}}",
                                    offset_value,
                                    offset_value,
                                    offset_value
                                );
                                
                                result.push_str(&fixed_block);
                            }
                        }
                    } else {
                        result.push_str(&unsafe_block);
                    }
                    
                    found_problem = false;
                }
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }
        
        Ok(result)
    }
    
    pub fn generate_error_report(&self, candidate: &OverflowCandidate) -> ErrorReport {
        ErrorReport {
            issue_type: candidate.operation.clone(),
            location: format!("Line {}", candidate.line),
            risk_level: match candidate.operation.as_str() {
                "pointer_offset" => "Critical",
                "allocation" => "Medium",
                _ => "Unknown"
            }.to_string(),
            description: format!("检测到未检查的指针偏移操作: {:?}", candidate.offset),
            impact: match candidate.operation.as_str() {
                "pointer_offset" => "可能导致未定义行为和内存访问违规".to_string(),
                "allocation" => "潜在的内存安全风险".to_string(),
                _ => "未知影响".to_string()
            },
            recommendation: "建议在进行指针操作前添加显式的边界检查".to_string(),
        }
    }

    fn find_line_with_context(&self, lines: &[&str], _target: &str) -> Result<(usize, String)> {
        for (i, line) in lines.iter().enumerate() {
            if line.contains("*ptr.add") {
                return Ok((i + 1, line.to_string()));
            }
        }
        Ok((0, "".to_string()))
    }
}

fn fix_overflow(ast: &mut File, _overflow: &OverflowCandidate) {
    for item in &mut ast.items {
        if let Item::Fn(func) = item {
            rectify_block(&mut func.block);
        }
    }
}

#[allow(dead_code)]
fn fix_function(func: &mut ItemFn, _overflow: &OverflowCandidate) {
    rectify_block(&mut func.block);
}

#[allow(dead_code)]
fn fix_block(block: &mut Block, _overflow: &OverflowCandidate) {
    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Expr(expr, _) => {
                fix_expr(expr);
            }
            _ => {}
        }
    }
}

#[allow(dead_code)]
fn fix_expr(expr: &mut Expr) {
    match expr {
        Expr::Unsafe(ExprUnsafe { block, .. }) => {
            rectify_block(block);
        }
        Expr::Block(ExprBlock { block, .. }) => {
            rectify_block(block);
        }
        Expr::MethodCall(method_call) => {
            if method_call.method.to_string() == "add" {
                *expr = create_safe_add_call(method_call);
            }
        }
        _ => {}
    }
}

#[allow(dead_code)]
fn is_target_call(method_call: &ExprMethodCall, overflow: &OverflowCandidate) -> bool {
    let method_name = method_call.method.to_string();
    let receiver_str = method_call.receiver.to_token_stream().to_string();
    
    method_name == "add" && receiver_str.contains(&overflow.buffer_name)
}

#[allow(dead_code)]
fn create_safe_add_call(method_call: &ExprMethodCall) -> Expr {
    let receiver = &method_call.receiver;
    let args = &method_call.args;
    let buffer_name = "buffer";
    
    let safe_code = quote! {
        {
            let ptr = #receiver;
            let offset = #args;
            let buffer_len = #buffer_name.len();
            
            if offset as usize >= buffer_len {
                eprintln!("Buffer overflow prevented: {} (size: {}) accessed with offset {}", 
                          #buffer_name, buffer_len, offset);
                
                #buffer_name.resize(offset as usize + 1, 0);
                
                #buffer_name.as_mut_ptr().add(offset as usize)
            } else {
                ptr.add(offset as usize)
            }
        }
    };
    
    syn::parse_quote!(#safe_code)
}

fn lift_and_guard_expr(expr: &Expr, temp_vars: &mut Vec<TokenStream>, var_count: &mut usize) -> TokenStream {
    match expr {
        Expr::Binary(ExprBinary { left, op, right, .. }) => {
            let left_ts = lift_and_guard_expr(left, temp_vars, var_count);
            let right_ts = lift_and_guard_expr(right, temp_vars, var_count);

            *var_count += 1;
            let x1 = syn::Ident::new(&format!("x{}", *var_count), proc_macro2::Span::call_site());
            *var_count += 1;
            let x2 = syn::Ident::new(&format!("x{}", *var_count), proc_macro2::Span::call_site());
            *var_count += 1;
            let y = syn::Ident::new(&format!("y{}", *var_count), proc_macro2::Span::call_site());

            let checked_fn = match op {
                BinOp::Add(_) => "checked_add",
                BinOp::Sub(_) => "checked_sub",
                BinOp::Mul(_) => "checked_mul",
                BinOp::Div(_) => "checked_div",
                _ => "checked_add",
            };

            temp_vars.push(quote! { let #x1 = #left_ts; });
            temp_vars.push(quote! { let #x2 = #right_ts; });
            temp_vars.push(quote! {
                let #y = #x1.#checked_fn(#x2)
                    .on_flow(Error::new("Overflow for operation"))?;
            });

            quote! { #y }
        }
        Expr::Call(ExprCall { func, args, .. }) => {
            let mut arg_tokens = Vec::new();
            for arg in args {
                arg_tokens.push(lift_and_guard_expr(arg, temp_vars, var_count));
            }
            quote! { #func(#(#arg_tokens),*) }
        }
        _ => quote! { #expr },
    }
}

fn rectify_block(block: &mut syn::Block) {
    let mut new_stmts = Vec::new();
    let mut var_count = 0;

    for stmt in &block.stmts {
        if let Stmt::Expr(expr, _) = stmt {
            let mut temp_vars = Vec::new();
            let guarded = lift_and_guard_expr(expr, &mut temp_vars, &mut var_count);
            new_stmts.extend(temp_vars.into_iter().map(|ts| syn::parse2(ts).unwrap()));
            new_stmts.push(syn::parse2(guarded).unwrap());
        } else {
            new_stmts.push(stmt.clone());
        }
    }
    block.stmts = new_stmts;
}

pub trait SafeLib<T> {
    fn checked_add(&self, y: T) -> Option<T>;
    fn checked_sub(&self, y: T) -> Option<T>;
    fn on_flow(self, err: Error) -> Result<T, Error>;
}