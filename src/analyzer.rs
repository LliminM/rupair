use syn::{File, Item, ItemFn, Block, Stmt, Expr, ExprUnsafe, ExprMethodCall, ExprPath, Pat, PatIdent, Local, visit::{self, Visit}};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct OverflowCandidate {
    pub location: String,
    pub buffer_name: String,
    pub operation: String,
    pub line: usize,
    pub column: usize,
}

// 缓冲区和指针的信息
#[derive(Debug, Clone)]
struct PointerInfo {
    buffer_name: String,
    is_ptr: bool,
}

pub fn find_buffer_overflows(ast: &File) -> Vec<OverflowCandidate> {
    let mut visitor = OverflowVisitor {
        candidates: Vec::new(),
        pointers: HashMap::new(),
        current_function: String::new(),
    };
    
    visitor.visit_file(ast);
    visitor.candidates
}

struct OverflowVisitor {
    candidates: Vec<OverflowCandidate>,
    pointers: HashMap<String, PointerInfo>,
    current_function: String,
}

impl<'ast> Visit<'ast> for OverflowVisitor {
    // 访问函数定义
    fn visit_item_fn(&mut self, func: &'ast ItemFn) {
        // 记录当前函数名
        self.current_function = func.sig.ident.to_string();
        
        // 继续访问函数体
        visit::visit_item_fn(self, func);
    }
    
    // 访问局部变量声明
    fn visit_local(&mut self, local: &'ast Local) {
        // 识别缓冲区声明，例如 let buffer = vec![0; 10]
        if let Pat::Ident(pat_ident) = &local.pat {
            let var_name = pat_ident.ident.to_string();
            
            // 检查是否是Vec或数组类型
            if let Some(init) = &local.init {
                match &*init.expr {
                    // 匹配 vec![]
                    Expr::Macro(expr_macro) if expr_macro.mac.path.is_ident("vec") => {
                        println!("Found vec! for {}", var_name);
                        self.pointers.insert(var_name.clone(), PointerInfo {
                            buffer_name: var_name.clone(),
                            is_ptr: false,
                        });
                    },
                    // 匹配 Vec::new() 等
                    Expr::Call(call) => {
                        if let Expr::Path(path) = &*call.func {
                            let is_vec = path.path.segments.iter().any(|seg| 
                                seg.ident == "vec" || seg.ident == "Vec"
                            );
                            
                            if is_vec {
                                println!("Found Vec for {}", var_name);
                                self.pointers.insert(var_name.clone(), PointerInfo {
                                    buffer_name: var_name.clone(),
                                    is_ptr: false,
                                });
                            }
                        }
                    },
                    _ => {}
                }
                
                // 处理指针赋值
                if let Expr::MethodCall(method_call) = &*init.expr {
                    let method_name = method_call.method.to_string();
                    
                    if method_name == "as_mut_ptr" || method_name == "as_ptr" {
                        if let Expr::Path(path) = &*method_call.receiver {
                            if let Some(ident) = path.path.get_ident() {
                                let buffer_name = ident.to_string();
                                println!("Found pointer {} from buffer {}", var_name, buffer_name);
                                
                                // 记录指针变量关联到源缓冲区
                                self.pointers.insert(var_name.clone(), PointerInfo {
                                    buffer_name,
                                    is_ptr: true,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        visit::visit_local(self, local);
    }
    
    // 访问方法调用
    fn visit_expr_method_call(&mut self, expr: &'ast ExprMethodCall) {
        let method_name = expr.method.to_string();
        
        // 检测可能导致溢出的 add 方法
        if method_name == "add" {
            if let Expr::Path(path) = &*expr.receiver {
                if let Some(ident) = path.path.get_ident() {
                    let ptr_name = ident.to_string();
                    println!("Found add for pointer {}", ptr_name);
                    
                    // 检查这个指针是否关联到某个缓冲区
                    if let Some(ptr_info) = self.pointers.get(&ptr_name) {
                        println!("Detected potential overflow for buffer {}", ptr_info.buffer_name);
                        
                        // 创建溢出候选项
                        self.candidates.push(OverflowCandidate {
                            location: format!("{}:{}", self.current_function, expr.method.span().start().line),
                            buffer_name: ptr_info.buffer_name.clone(),
                            operation: "pointer_offset".to_string(),
                            line: expr.method.span().start().line,
                            column: expr.method.span().start().column,
                        });
                    }
                }
            }
        }
        
        visit::visit_expr_method_call(self, expr);
    }
    
    // 访问不安全块
    fn visit_expr_unsafe(&mut self, expr: &'ast ExprUnsafe) {
        // 特别关注unsafe块中的内容
        println!("Found unsafe block");
        visit::visit_expr_unsafe(self, expr);
    }
    
    // 默认继续遍历
    fn visit_expr(&mut self, expr: &'ast Expr) {
        visit::visit_expr(self, expr);
    }
}