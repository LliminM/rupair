use crate::analyzer::OverflowCandidate;
use quote::quote;
use syn::{File, Item, ItemFn, Stmt, Expr, ExprUnsafe, ExprBlock, ExprMethodCall, Block};
use quote::ToTokens;

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