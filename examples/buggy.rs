fn main() {
    let mut v = vec![1, 2, 3];
    unsafe {
        let p = v.as_mut_ptr();
        *p.add(10) = 42; // buffer overflow!
    }
}