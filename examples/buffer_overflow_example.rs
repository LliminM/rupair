fn main() {
    // 创建一个大小为10的缓冲区
    let mut buffer = vec![0u8; 10];
    
    // 获取缓冲区的可变指针
    let ptr = buffer.as_mut_ptr();
    
    // 创建一些测试数据
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    
    println!("Buffer size: {}", buffer.len());
    println!("Data size: {}", data.len());
    
    // 尝试复制数据到缓冲区，可能导致缓冲区溢出
    unsafe {
        for (i, &value) in data.iter().enumerate() {
            // 以下行可能导致缓冲区溢出，因为data.len() > buffer.len()
            *ptr.add(i) = value;
        }
    }
    
    // 打印结果
    println!("Modified buffer: {:?}", buffer);
} 