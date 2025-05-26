// 修复问题 #1
// Please implement safe buffer handling for this case

// 修复问题 #2
// Fixed version using safe alternatives
use std::error::Error;

fn safe_buffer_access(buffer: &mut Vec<u8>, index: usize, value: u8) -> Result<(), Box<dyn Error>> {
    if index >= buffer.len() {
        return Err("Index out of bounds".into());
    }
    buffer[index] = value;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut buffer = vec![0; 5];
    safe_buffer_access(&mut buffer, 2, 42)?;
    println!("Buffer accessed safely");
    Ok(())
}

// 修复问题 #3
// Fixed version with proper bounds checking and error handling
use std::error::Error;

fn process_data(data: &[u8], buffer: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {
    // Ensure buffer has enough capacity
    if buffer.len() < data.len() {
        buffer.resize(data.len(), 0);
    }

    // Safe copy with bounds checking
    for (i, &item) in data.iter().enumerate() {
        if i < buffer.len() {
            buffer[i] = item;
        } else {
            return Err("Buffer capacity exceeded".into());
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let data = vec![1, 2, 3, 4, 5];
    let mut buffer = Vec::with_capacity(data.len());
    process_data(&data, &mut buffer)?;
    println!("Data processed successfully");
    Ok(())
}

// 修复问题 #4
// Please implement safe buffer handling for this case

