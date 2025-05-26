fn test_buffer_overflow() {
    // 1. Vector index overflow
    let mut buffer = vec![0; 5];
    for i in 0..10 {
        buffer[i] = i as u8;  // Potential overflow
    }

    // 2. Unsafe pointer arithmetic overflow
    let ptr = buffer.as_mut_ptr();
    unsafe {
        *ptr.add(8) = 42;  // Access beyond buffer bounds
    }

    // 3. Vector with insufficient capacity
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mut small_buffer = vec![0; 3];
    for (i, &item) in data.iter().enumerate() {
        small_buffer[i] = item;  // Will overflow
    }

    // 4. Raw pointer offset overflow
    let another_ptr = small_buffer.as_mut_ptr();
    unsafe {
        let val = *another_ptr.offset(5);  // Access beyond bounds
        println!("Value: {}", val);
    }
}

fn main() {
    test_buffer_overflow();
} 