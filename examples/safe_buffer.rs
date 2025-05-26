use std::error::Error;

/// 安全的缓冲区操作示例
fn safe_buffer_operations() -> Result<(), Box<dyn Error>> {
    // 1. 安全的向量操作 - 使用 with_capacity 预分配空间
    let data = vec![1, 2, 3, 4, 5];
    let mut buffer = Vec::with_capacity(data.len());
    
    // 使用 extend 安全地复制数据
    buffer.extend_from_slice(&data);
    println!("安全的向量复制完成: {:?}", buffer);

    // 2. 安全的迭代访问 - 使用 iter().enumerate() 和边界检查
    for (i, &item) in data.iter().enumerate() {
        if i < buffer.len() {
            buffer[i] = item + 1;
        }
    }
    println!("安全的迭代修改完成: {:?}", buffer);

    // 3. 安全的切片操作
    let slice = &data[..];
    let safe_window = &slice[..slice.len().min(3)];  // 安全地获取前3个元素或更少
    println!("安全的切片操作完成: {:?}", safe_window);

    // 4. 使用 get 进行安全的索引访问
    if let Some(value) = buffer.get(2) {
        println!("安全的索引访问: {}", value);
    }

    // 5. 安全的缓冲区扩展
    let more_data = vec![6, 7, 8];
    buffer.reserve(more_data.len());  // 确保有足够空间
    buffer.extend_from_slice(&more_data);
    println!("安全的缓冲区扩展完成: {:?}", buffer);

    // 6. 使用 chunks 进行安全的批量处理
    for chunk in buffer.chunks(2) {
        println!("安全的块处理: {:?}", chunk);
    }

    // 7. 安全的缓冲区清理
    buffer.clear();
    buffer.shrink_to_fit();  // 释放多余的容量
    println!("安全的缓冲区清理完成: {:?}", buffer);

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("开始执行安全的缓冲区操作示例...\n");
    safe_buffer_operations()?;
    println!("\n所有操作已安全完成！");
    Ok(())
} 