use z3::{Context, Solver, ast::Int};

use crate::analyzer::OverflowCandidate;

#[derive(Debug)]
pub struct BufferConstraint {
    pub buffer_size: u64,
    pub offset: u64,
    pub is_overflow: bool,
}

#[derive(Clone)]
pub struct BufferSolver<'a> {
    ctx: &'a Context,
    solver: Solver<'a>,
    buffer_vars: std::collections::HashMap<String, Int<'a>>,
}

impl<'a> BufferSolver<'a> {
    pub fn new(ctx: &'a Context) -> Self {
        let solver = Solver::new(ctx);
        BufferSolver {
            ctx,
            solver,
            buffer_vars: std::collections::HashMap::new(),
        }
    }

    pub fn add_buffer(&mut self, name: &str, size: u64) {
        let size_ast = Int::from_i64(self.ctx, size as i64);
        self.buffer_vars.insert(name.to_string(), size_ast);
    }

    pub fn check_overflow(&mut self, candidate: &OverflowCandidate) -> BufferConstraint {
        let buffer_size = candidate.buffer_size.unwrap_or(0) as u64;
        let offset = candidate.offset.unwrap_or(0) as u64;
        
        if buffer_size == 0 || offset == 0 {
            return BufferConstraint {
                buffer_size,
                offset,
                is_overflow: true,
            };
        }
        
        let buffer_size_ast = Int::from_i64(self.ctx, buffer_size as i64);
        let offset_ast = Int::from_i64(self.ctx, offset as i64);
        
        let overflow_condition = offset_ast.ge(&buffer_size_ast);
        
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
}