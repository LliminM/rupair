fn main() {
    println!("cargo:rustc-link-search=D:/OneDrive/桌面/rust/build/x86_64-pc-windows-msvc/stage0-rustc/x86_64-pc-windows-msvc/release/deps");
    println!("cargo:rustc-link-lib=static=rustc_driver");
    println!("cargo:rustc-link-arg=/FORCE:MULTIPLE");
}