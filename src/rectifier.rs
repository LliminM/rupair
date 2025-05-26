use crate::analyzer::OverflowCandidate;
use quote::quote;
use syn::{File, Item, ItemFn, Stmt, Expr, ExprUnsafe, ExprBlock, ExprMethodCall, Block};
use quote::ToTokens;
use std::path::PathBuf;
use anyhow::Result;
use std::fs;

/// 修复代码中的缓冲区溢出问题
pub fn rectify(ast: &File, overflows: &[OverflowCandidate]) -> String {
    if overflows.is_empty() {
        return quote!(#ast).to_string();
    }

    // 创建修复后的AST
    let mut fixed_ast = ast.clone();

    // 修复每个溢出点
    for overflow in overflows {
        fix_overflow(&mut fixed_ast, overflow);
    }

    // 将AST转换为代码字符串
    quote!(#fixed_ast).to_string()
}

/// 修复单个溢出点
fn fix_overflow(ast: &mut File, overflow: &OverflowCandidate) {
    for item in &mut ast.items {
        if let Item::Fn(func) = item {
            fix_function(func, overflow);
        }
    }
}

/// 修复函数中的溢出
fn fix_function(func: &mut ItemFn, overflow: &OverflowCandidate) {
    fix_block(&mut *func.block, overflow);
}

/// 修复代码块中的溢出
fn fix_block(block: &mut Block, overflow: &OverflowCandidate) {
    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Expr(expr, _) => {
                fix_expr(expr, overflow);
            }
            _ => {}
        }
    }
}

/// 修复表达式中的溢出
fn fix_expr(expr: &mut Expr, overflow: &OverflowCandidate) {
    match expr {
        Expr::Unsafe(ExprUnsafe { block, .. }) => {
            fix_block(block, overflow);
        }
        Expr::Block(ExprBlock { block, .. }) => {
            fix_block(block, overflow);
        }
        Expr::MethodCall(method_call) => {
            if method_call.method.to_string() == "add" &&
               is_target_call(method_call, overflow) {
                *expr = create_safe_add_call(method_call, overflow);
            }
        }
        _ => {}
    }
}

fn is_target_call(method_call: &ExprMethodCall, overflow: &OverflowCandidate) -> bool {
    // 检查方法名是否为 "add" 且 receiver 包含 buffer_name
    let method_name = method_call.method.to_string();
    let receiver_str = method_call.receiver.to_token_stream().to_string();
    
    method_name == "add" && receiver_str.contains(&overflow.buffer_name)
}

/// 创建安全的add调用（带边界检查和自动调整）
fn create_safe_add_call(method_call: &ExprMethodCall, overflow: &OverflowCandidate) -> Expr {
    let receiver = &method_call.receiver;
    let args = &method_call.args;
    let buffer_name = &overflow.buffer_name;
    
    // 创建带边界检查和自动调整的安全代码
    let safe_code = quote! {
        {
            let ptr = #receiver;
            let offset = #args;
            let buffer_len = #buffer_name.len();
            
            // 如果偏移量超出范围，调整缓冲区大小
            if offset as usize >= buffer_len {
                // 记录日志
                eprintln!("Buffer overflow prevented: {} (size: {}) accessed with offset {}", 
                          #buffer_name, buffer_len, offset);
                
                // 调整缓冲区大小
                #buffer_name.resize(offset as usize + 1, 0);
                
                // 使用新的指针
                #buffer_name.as_mut_ptr().add(offset as usize)
            } else {
                // 安全范围内，允许操作
                ptr.add(offset as usize)
            }
        }
    };
    
    syn::parse_quote!(#safe_code)
}

#[derive(Debug)]
pub struct CodeFix {
    pub original_code: String,
    pub fixed_code: String,
    pub location: String,
    pub fix_type: FixType,
}

#[derive(Debug)]
pub enum FixType {
    BoundCheck,
    VecResize,
    SafeAccess,
    UnsafeToSafe,
}

pub struct Rectifier {
    source_file: PathBuf,
}

impl Rectifier {
    pub fn new(source_file: PathBuf) -> Self {
        Self { source_file }
    }

    pub fn generate_fix(&self, overflow: &super::mir_analyzer::BufferOverflow) -> Result<CodeFix> {
        let content = fs::read_to_string(&self.source_file)?;
        let ast = syn::parse_file(&content)?;

        let fix = match overflow.operation_type.as_str() {
            "Buffer Overflow" => self.fix_buffer_overflow(&overflow.location),
            "Pointer Arithmetic" => self.fix_pointer_arithmetic(&overflow.location),
            "Unsafe Block" => self.fix_unsafe_block(&overflow.location),
            _ => self.generate_generic_fix(&overflow.location),
        }?;

        Ok(fix)
    }

    fn fix_buffer_overflow(&self, location: &str) -> Result<CodeFix> {
        // Extract the problematic code context
        let content = fs::read_to_string(&self.source_file)?;
        let lines: Vec<&str> = content.lines().collect();
        
        // Find the relevant code section
        let (line_num, line) = self.find_line_with_context(&lines, location)?;
        
        if line.contains("for") && line.contains("enumerate") {
            // Add bounds checking to for loop
            let fixed_code = format!(
                "for (i, item) in data.iter().enumerate() {{
                    if i < buffer.len() {{
                        buffer[i] = *item;
                    }} else {{
                        break;
                    }}
                }}"
            );

            Ok(CodeFix {
                original_code: line.to_string(),
                fixed_code,
                location: format!("Line {}", line_num),
                fix_type: FixType::BoundCheck,
            })
        } else if line.contains("vec!") {
            // Fix vector initialization
            let fixed_code = if line.contains("vec![") {
                line.replace("vec![", "Vec::with_capacity(")
                    .replace("]", ")")
            } else {
                format!("let mut buffer = Vec::with_capacity(required_size);")
            };

            Ok(CodeFix {
                original_code: line.to_string(),
                fixed_code,
                location: format!("Line {}", line_num),
                fix_type: FixType::VecResize,
            })
        } else {
            self.generate_generic_fix(location)
        }
    }

    fn fix_pointer_arithmetic(&self, location: &str) -> Result<CodeFix> {
        let content = fs::read_to_string(&self.source_file)?;
        let lines: Vec<&str> = content.lines().collect();
        let (line_num, line) = self.find_line_with_context(&lines, location)?;

        // Convert unsafe pointer arithmetic to safe slice operations
        let fixed_code = if line.contains("ptr.add") {
            line.replace("*ptr.add(i)", "&buffer[i]")
        } else if line.contains("offset") {
            line.replace(".offset(", ".get(")
        } else {
            format!("buffer.get(index).copied().unwrap_or_default()")
        };

        Ok(CodeFix {
            original_code: line.to_string(),
            fixed_code,
            location: format!("Line {}", line_num),
            fix_type: FixType::SafeAccess,
        })
    }

    fn fix_unsafe_block(&self, location: &str) -> Result<CodeFix> {
        let content = fs::read_to_string(&self.source_file)?;
        let lines: Vec<&str> = content.lines().collect();
        let (line_num, line) = self.find_line_with_context(&lines, location)?;

        // Convert unsafe block to safe code
        let fixed_code = if line.contains("unsafe") {
            line.replace("unsafe {", "")
                .replace("*ptr.add(i)", "buffer.get(i).copied().unwrap_or_default()")
        } else {
            format!("let value = buffer.get(index).copied().unwrap_or_default();")
        };

        Ok(CodeFix {
            original_code: line.to_string(),
            fixed_code,
            location: format!("Line {}", line_num),
            fix_type: FixType::UnsafeToSafe,
        })
    }

    fn generate_generic_fix(&self, location: &str) -> Result<CodeFix> {
        Ok(CodeFix {
            original_code: location.to_string(),
            fixed_code: format!(
                "// TODO: Add appropriate bounds checking
if index < buffer.len() {{
    // Perform operation
}} else {{
    // Handle out of bounds case
}}"
            ),
            location: location.to_string(),
            fix_type: FixType::BoundCheck,
        })
    }

    fn find_line_with_context(&self, lines: &[&str], target: &str) -> Result<(usize, String)> {
        for (i, line) in lines.iter().enumerate() {
            if line.contains(target) {
                return Ok((i + 1, line.to_string()));
            }
        }
        Ok((0, target.to_string()))
    }

    pub fn generate_complete_fix(&self, overflow: &super::mir_analyzer::BufferOverflow) -> Result<String> {
        let content = fs::read_to_string(&self.source_file)?;
        let ast = syn::parse_file(&content)?;

        match overflow.operation_type.as_str() {
            "Buffer Overflow" => {
                let mut fixed_code = String::from(
                    "// Fixed version with proper bounds checking and error handling
use std::error::Error;

fn process_data(data: &[u8], buffer: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {
    // Ensure buffer has enough capacity
    if buffer.len() < data.len() {
        buffer.resize(data.len(), 0);
    }

    // Safe copy with bounds checking
    for (i, &item) in data.iter().enumerate() {
        if i < buffer.len() {
            buffer[i] = item;
        } else {
            return Err(\"Buffer capacity exceeded\".into());
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let data = vec![1, 2, 3, 4, 5];
    let mut buffer = Vec::with_capacity(data.len());
    process_data(&data, &mut buffer)?;
    println!(\"Data processed successfully\");
    Ok(())
}\n"
                );
                Ok(fixed_code)
            },
            "Pointer Arithmetic" => {
                let mut fixed_code = String::from(
                    "// Fixed version using safe alternatives
use std::error::Error;

fn safe_buffer_access(buffer: &mut Vec<u8>, index: usize, value: u8) -> Result<(), Box<dyn Error>> {
    if index >= buffer.len() {
        return Err(\"Index out of bounds\".into());
    }
    buffer[index] = value;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut buffer = vec![0; 5];
    safe_buffer_access(&mut buffer, 2, 42)?;
    println!(\"Buffer accessed safely\");
    Ok(())
}\n"
                );
                Ok(fixed_code)
            },
            _ => Ok(String::from("// Please implement safe buffer handling for this case\n"))
        }
    }

    pub fn generate_error_report(&self, overflow: &super::mir_analyzer::BufferOverflow) -> ErrorReport {
        ErrorReport {
            issue_type: overflow.operation_type.clone(),
            location: overflow.location.clone(),
            risk_level: match overflow.operation_type.as_str() {
                "Buffer Overflow" => "High",
                "Pointer Arithmetic" => "Critical",
                "Unsafe Block" => "Medium",
                _ => "Unknown"
            }.to_string(),
            description: overflow.description.clone(),
            impact: match overflow.operation_type.as_str() {
                "Buffer Overflow" => "可能导致内存损坏、程序崩溃或安全漏洞".to_string(),
                "Pointer Arithmetic" => "可能导致未定义行为和内存访问违规".to_string(),
                "Unsafe Block" => "潜在的内存安全风险".to_string(),
                _ => "未知影响".to_string()
            },
            recommendation: format!("建议修复方案:\n{}", overflow.fix_suggestion),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;
    
    #[test]
    fn test_rectify_simple_overflow() {
        let ast: File = parse_quote! {
            fn test_function() {
                let mut buffer = vec![0u8; 10];
                let ptr = buffer.as_mut_ptr();
                unsafe {
                    let val = ptr.add(15);
                }
            }
        };
        
        let overflow = OverflowCandidate {
            location: "buffer:5".to_string(),
            buffer_name: "buffer".to_string(),
            operation: "pointer_offset".to_string(),
            line: 5,
            column: 30,
            buffer_size: Some(10),
            offset: Some(15),
        };
        
        let result = rectify(&ast, &[overflow]);
        
        // 验证结果包含边界检查和自动调整
        assert!(result.contains("if offset as usize >= buffer_len"));
        assert!(result.contains("resize"));
        assert!(result.contains("eprintln!"));
    }
}