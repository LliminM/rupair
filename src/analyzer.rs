use syn::{File, ItemFn, Expr, ExprUnsafe, ExprMethodCall, Pat, Local, visit::{self, Visit}, Lit, ExprLit};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct OverflowCandidate {
    pub location: String,
    pub buffer_name: String,
    pub operation: String,
    pub line: usize,
    pub column: usize,
    pub buffer_size: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone)]
struct PointerInfo {
    buffer_name: String,
    buffer_size: Option<usize>,
}

pub fn find_buffer_overflows(ast: &File, mir_candidates: Vec<OverflowCandidate>) -> Vec<OverflowCandidate> {
    let mut visitor = OverflowVisitor {
        candidates: Vec::new(),
        pointers: HashMap::new(),
        current_function: String::new(),
    };
    
    visitor.visit_file(ast);
    visitor.candidates.extend(mir_candidates);
    visitor.candidates
}

struct OverflowVisitor {
    candidates: Vec<OverflowCandidate>,
    pointers: HashMap<String, PointerInfo>,
    current_function: String,
}

impl<'ast> Visit<'ast> for OverflowVisitor {
    fn visit_item_fn(&mut self, func: &'ast ItemFn) {
        self.current_function = func.sig.ident.to_string();
        visit::visit_item_fn(self, func);
    }
    
    fn visit_local(&mut self, local: &'ast Local) {
        if let Pat::Ident(pat_ident) = &local.pat {
            let var_name = pat_ident.ident.to_string();
            
            if let Some(init) = &local.init {
                if let Expr::Macro(expr_macro) = &*init.expr {
                    if expr_macro.mac.path.is_ident("vec") {
                        let size = extract_vec_size(&expr_macro.mac);
                        println!("Found vec! for {} with size {:?}", var_name, size);
                        self.pointers.insert(var_name.clone(), PointerInfo {
                            buffer_name: var_name.clone(),
                            buffer_size: size,
                        });
                    }
                }
                
                if let Expr::MethodCall(method_call) = &*init.expr {
                    let method_name = method_call.method.to_string();
                    
                    if method_name == "as_mut_ptr" || method_name == "as_ptr" {
                        if let Expr::Path(path) = &*method_call.receiver {
                            if let Some(ident) = path.path.get_ident() {
                                let buffer_name = ident.to_string();
                                let buffer_size = self.pointers.get(&buffer_name)
                                    .and_then(|info| info.buffer_size);
                                
                                println!("Found pointer {} from buffer {} (size: {:?})", 
                                       var_name, buffer_name, buffer_size);
                                
                                self.pointers.insert(var_name.clone(), PointerInfo {
                                    buffer_name,
                                    buffer_size,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        visit::visit_local(self, local);
    }
    
    fn visit_expr_method_call(&mut self, expr: &'ast ExprMethodCall) {
        let method_name = expr.method.to_string();
        
        if method_name == "add" {
            if let Expr::Path(path) = &*expr.receiver {
                if let Some(ident) = path.path.get_ident() {
                    let ptr_name = ident.to_string();
                    
                    if let Some(ptr_info) = self.pointers.get(&ptr_name) {
                        let offset = extract_offset(&expr.args);
                        
                        println!("Found add for pointer {} with offset {:?}", ptr_name, offset);
                        
                        let line = 0;
                        let column = 0;
                        
                        self.candidates.push(OverflowCandidate {
                            location: self.current_function.clone(),
                            buffer_name: ptr_info.buffer_name.clone(),
                            operation: "pointer_offset".to_string(),
                            line,
                            column,
                            buffer_size: ptr_info.buffer_size,
                            offset,
                        });
                    }
                }
            }
        }
        
        visit::visit_expr_method_call(self, expr);
    }
    
    fn visit_expr_unsafe(&mut self, expr: &'ast ExprUnsafe) {
        println!("Found unsafe block");
        visit::visit_expr_unsafe(self, expr);
    }
    
    fn visit_expr(&mut self, expr: &'ast Expr) {
        visit::visit_expr(self, expr);
    }
}

fn extract_vec_size(mac: &syn::Macro) -> Option<usize> {
    if let syn::MacroDelimiter::Bracket(_) = mac.delimiter {
        if let Ok(tokens) = syn::parse2::<syn::Expr>(mac.tokens.clone()) {
            if let syn::Expr::Array(array) = tokens {
                return Some(array.elems.len());
            }
        }
    }
    None
}

fn extract_offset(args: &syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>) -> Option<usize> {
    if let Some(arg) = args.first() {
        if let syn::Expr::Lit(ExprLit { lit: Lit::Int(lit), .. }) = arg {
            return lit.base10_parse().ok();
        }
    }
    None
}