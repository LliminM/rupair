use rupair::solver::BufferSolver;
use z3::Context;

#[test]
fn test_safe_buffer_access() {
    let ctx = Context::new(&z3::Config::new());
    let mut solver = BufferSolver::new(&ctx);
    solver.add_buffer("safe_buffer", 10);
    
    // 测试安全的访问
    let result = solver.check_overflow("safe_buffer", 5);
    assert!(!result.is_overflow);
}

#[test]
fn test_buffer_overflow() {
    let ctx = Context::new(&z3::Config::new());
    let mut solver = BufferSolver::new(&ctx);
    solver.add_buffer("overflow_buffer", 10);
    
    // 测试溢出访问
    let result = solver.check_overflow("overflow_buffer", 15);
    assert!(result.is_overflow);
}

#[test]
fn test_edge_case() {
    let ctx = Context::new(&z3::Config::new());
    let mut solver = BufferSolver::new(&ctx);
    solver.add_buffer("edge_buffer", 10);
    
    // 测试边界情况
    let result = solver.check_overflow("edge_buffer", 10);
    assert!(result.is_overflow); // 因为索引从0开始，所以10也是溢出
}

#[test]
fn test_multiple_buffers() {
    let ctx = Context::new(&z3::Config::new());
    let mut solver = BufferSolver::new(&ctx);
    solver.add_buffer("buffer1", 5);
    solver.add_buffer("buffer2", 15);
    
    // 测试不同大小的缓冲区
    let result1 = solver.check_overflow("buffer1", 6);
    let result2 = solver.check_overflow("buffer2", 6);
    
    assert!(result1.is_overflow);
    assert!(!result2.is_overflow);
}

#[test]
fn test_generated_test_cases() {
    let ctx = Context::new(&z3::Config::new());
    let mut solver = BufferSolver::new(&ctx);
    solver.add_buffer("test_buffer", 8);
    
    // 生成并打印测试用例
    let test_case = solver.generate_test_case("test_buffer", 12);
    println!("Generated test case:\n{}", test_case);
    
    // 验证生成的测试用例
    assert!(test_case.contains("vec![0u8; 8]"));
    assert!(test_case.contains("ptr.add(12)"));
} 