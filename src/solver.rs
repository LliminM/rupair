use z3::{Context, Solver, ast::Int};
use std::collections::HashMap;
use crate::analyzer::OverflowCandidate;

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

    pub fn check_overflow(&mut self, candidate: &OverflowCandidate) -> BufferConstraint {
        // 使用从代码中提取的实际缓冲区大小
        let buffer_size = candidate.buffer_size.unwrap_or(0) as u64;
        let offset = candidate.offset.unwrap_or(0) as u64;
        
        // 如果无法确定大小或偏移量，返回保守的结果
        if buffer_size == 0 || offset == 0 {
            return BufferConstraint {
                buffer_size,
                offset,
                is_overflow: true, // 保守估计：无法确定时假设可能溢出
            };
        }
        
        let buffer_size_ast = Int::from_i64(self.ctx, buffer_size as i64);
        let offset_ast = Int::from_i64(self.ctx, offset as i64);
        
        // 创建约束：offset >= buffer_size
        let overflow_condition = offset_ast.ge(&buffer_size_ast);
        
        // 检查是否存在满足条件的解
        self.solver.push();
        self.solver.assert(&overflow_condition);
        let is_overflow = self.solver.check() == z3::SatResult::Sat;
        self.solver.pop(1);

        BufferConstraint {
            buffer_size,
            offset,
            is_overflow,
        }
    }

    pub fn generate_test_case(&self, candidate: &OverflowCandidate) -> String {
        let buffer_size = candidate.buffer_size.unwrap_or(10) as u64;
        let offset = candidate.offset.unwrap_or(15) as u64;

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
            candidate.buffer_name,
            buffer_size,
            offset,
            offset
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::OverflowCandidate;

    #[test]
    fn test_buffer_overflow_detection() {
        let ctx = Context::new(&z3::Config::new());
        let mut solver = BufferSolver::new(&ctx);
        
        // 创建测试用例
        let candidate = OverflowCandidate {
            location: "test:1".to_string(),
            buffer_name: "test_buffer".to_string(),
            operation: "pointer_offset".to_string(),
            line: 1,
            column: 1,
            buffer_size: Some(10),
            offset: Some(15),
        };
        
        // 检查溢出
        let result = solver.check_overflow(&candidate);
        assert!(result.is_overflow);
        assert_eq!(result.buffer_size, 10);
        assert_eq!(result.offset, 15);
        
        // 生成测试用例
        let test_case = solver.generate_test_case(&candidate);
        println!("Generated test case:\n{}", test_case);
    }
}