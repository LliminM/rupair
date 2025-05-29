fn main() {
    let mut buffer = vec![0u8; 10];
    if 15 < buffer.len() {
    buffer[15] = 42;
} else {
    panic!("Buffer overflow prevented: index 15");
}
}