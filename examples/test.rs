fn main() {
    let mut buffer = vec![0u8; 10];
    unsafe {
        let ptr = buffer.as_mut_ptr();
        *ptr.add(15) = 42; // 潜在溢出
    }
}