// 这个例子模拟了更复杂的缓冲区溢出场景
fn main() {
    // 创建两个不同大小的缓冲区
    let mut small_buffer = vec![0u8; 5];
    let mut large_buffer = vec![0u8; 20];
    
    // 获取指针
    let small_ptr = small_buffer.as_mut_ptr();
    let large_ptr = large_buffer.as_mut_ptr();
    
    // 偏移量
    let offsets = [0, 2, 4, 6, 8, 10, 12];
    
    // 条件控制的不安全访问
    unsafe {
        for &offset in &offsets {
            if offset < small_buffer.len() {
                // 安全访问
                *small_ptr.add(offset) = (offset * 2) as u8;
            } else {
                // 潜在的缓冲区溢出
                if offset % 2 == 0 {
                    // 这里会导致small_buffer溢出
                    *small_ptr.add(offset) = 99;
                } else {
                    // 这里对large_buffer是安全的
                    *large_ptr.add(offset) = 88;
                }
            }
        }
    }
    
    // 函数内部的缓冲区溢出
    process_buffer(&mut small_buffer);
    
    println!("Small buffer: {:?}", small_buffer);
    println!("Large buffer: {:?}", large_buffer);
}

fn process_buffer(buffer: &mut Vec<u8>) {
    let ptr = buffer.as_mut_ptr();
    
    // 潜在的溢出，访问索引8，但buffer大小只有5
    unsafe {
        *ptr.add(8) = 255; // 这里会导致溢出
    }
} 