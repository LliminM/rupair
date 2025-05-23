use crate::analyzer::OverflowCandidate;
use quote::quote;
use syn::{File, Item, ItemFn, Stmt, Expr, ExprUnsafe, ExprBlock, ExprMethodCall, Block};

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
            // 遍历并修复unsafe块
            fix_block(block, overflow);
        }
        Expr::Block(ExprBlock { block, .. }) => {
            // 遍历并修复普通代码块
            fix_block(block, overflow);
        }
        Expr::MethodCall(method_call) => {
            // 检查是否是要修复的方法调用
            if method_call.method.to_string() == "add" &&
               is_target_call(method_call, overflow) {
                // 替换为安全的方法调用
                *expr = create_safe_add_call(method_call, overflow);
            }
        }
        _ => {}
    }
}

/// 检查方法调用是否是目标溢出点
fn is_target_call(method_call: &ExprMethodCall, overflow: &OverflowCandidate) -> bool {
    // 根据位置信息判断
    let span = method_call.method.span();
    let line = span.start().line;
    let column = span.start().column;
    
    line == overflow.line && column == overflow.column
}

/// 创建安全的add调用（带边界检查）
fn create_safe_add_call(method_call: &ExprMethodCall, overflow: &OverflowCandidate) -> Expr {
    // 提取原始参数
    let receiver = &method_call.receiver;
    let args = &method_call.args;
    
    // 创建带边界检查的安全代码
    let buffer_name = &overflow.buffer_name;
    let safe_code = quote! {
        {
            let ptr = #receiver;
            let offset = #args;
            let buffer_len = #buffer_name.len();
            
            // 添加边界检查
            if offset as usize >= buffer_len {
                // 溢出处理：记录日志并使用安全的边界值
                eprintln!("Buffer overflow prevented: {} (size: {}) accessed with offset {}", 
                          #buffer_name, buffer_len, offset);
                
                // 使用最大安全偏移量替代
                ptr.add((buffer_len - 1) as usize)
            } else {
                // 安全范围内，允许操作
                ptr.add(offset as usize)
            }
        }
    };
    
    // 将TokenStream转换为Expr
    syn::parse_quote!(#safe_code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;
    
    #[test]
    fn test_rectify_simple_overflow() {
        // 创建测试AST
        let ast: File = parse_quote! {
            fn test_function() {
                let mut buffer = vec![0u8; 10];
                let ptr = buffer.as_mut_ptr();
                unsafe {
                    let val = ptr.add(15);
                }
            }
        };
        
        // 创建溢出信息
        let overflow = OverflowCandidate {
            location: "buffer:5".to_string(),
            buffer_name: "buffer".to_string(),
            operation: "pointer_offset".to_string(),
            line: 5, // 假设在第5行
            column: 30, // 假设在第30列
        };
        
        // 执行修复
        let result = rectify(&ast, &[overflow]);
        
        // 验证结果包含边界检查
        assert!(result.contains("if offset as usize >= buffer_len"));
        assert!(result.contains("eprintln!"));
    }
}