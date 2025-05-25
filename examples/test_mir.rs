fn main() {
    let mut buffer = vec![0u8; 10];
    let ptr = buffer.as_mut_ptr();
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    unsafe {
        for (i, &value) in data.iter().enumerate() {
            *ptr.add(i) = value;
        }
    }
}