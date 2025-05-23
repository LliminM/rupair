use z3::{Context, Solver, ast::Int};
use std::collections::HashMap;

#[derive(Debug)]
pub struct BufferConstraint {
    pub buffer_size: u64,
    pub offset: u64,
    pub is_overflow: bool,
}

pub struct BufferSolver<'a> {
    ctx: &'a Context,
    solver: Solver<'a>,
    buffer_vars: HashMap<String, Int<'a>>,
}

impl<'a> BufferSolver<'a> {
    pub fn new(ctx: &'a Context) -> Self {
        let solver = Solver::new(ctx);
        BufferSolver {
            ctx,
            solver,
            buffer_vars: HashMap::new(),
        }
    }

    pub fn add_buffer(&mut self, name: &str, size: u64) {
        let size_ast = Int::from_i64(self.ctx, size as i64);
        self.buffer_vars.insert(name.to_string(), size_ast);
    }

    pub fn check_overflow(&mut self, buffer_name: &str, offset: u64) -> BufferConstraint {
        let buffer_size = self.buffer_vars.get(buffer_name)
            .expect("Buffer not found")
            .clone();
        
        let offset_ast = Int::from_i64(self.ctx, offset as i64);
        
        // 创建约束：offset >= buffer_size
        let overflow_condition = offset_ast.ge(&buffer_size);
        
        // 检查是否存在满足条件的解
        self.solver.push();
        self.solver.assert(&overflow_condition);
        let is_overflow = self.solver.check() == z3::SatResult::Sat;
        self.solver.pop(1);

        BufferConstraint {
            buffer_size: buffer_size.as_i64().unwrap() as u64,
            offset,
            is_overflow,
        }
    }

    pub fn generate_test_case(&self, buffer_name: &str, offset: u64) -> String {
        let buffer_size = self.buffer_vars.get(buffer_name)
            .expect("Buffer not found")
            .as_i64()
            .unwrap() as u64;

        format!(
            r#"#[test]
fn test_buffer_overflow_{}() {{
    let mut buffer = vec![0u8; {}];
    let ptr = buffer.as_mut_ptr();
    unsafe {{
        // 尝试访问偏移量 {} 的位置
        let _ = ptr.add({});
    }}
}}"#,
            buffer_name,
            buffer_size,
            offset,
            offset
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_overflow_detection() {
        let ctx = Context::new(&z3::Config::new());
        let mut solver = BufferSolver::new(&ctx);
        
        // 添加一个大小为 10 的缓冲区
        solver.add_buffer("test_buffer", 10);
        
        // 检查偏移量 5（安全）
        let result1 = solver.check_overflow("test_buffer", 5);
        assert!(!result1.is_overflow);
        
        // 检查偏移量 15（溢出）
        let result2 = solver.check_overflow("test_buffer", 15);
        assert!(result2.is_overflow);
        
        // 生成测试用例
        let test_case = solver.generate_test_case("test_buffer", 15);
        println!("Generated test case:\n{}", test_case);
    }
}