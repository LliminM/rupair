use std::path::Path;

fn main() {
    // Debug: Print current directory
    println!("cargo:warning=Current directory: {:?}", std::env::current_dir().unwrap());

    let z3_include = r#"C:\Program Files\Z3\include"#;
    let z3_bin = r#"C:\Program Files\Z3\bin"#;

    // Debug: Check if files exist
    let z3_h_path = Path::new(z3_include).join("z3.h");
    println!("cargo:warning=Checking for z3.h at: {:?}", z3_h_path);
    if z3_h_path.exists() {
        println!("cargo:warning=Found z3.h!");
    } else {
        println!("cargo:warning=z3.h not found!");
    }

    // Set include directory
    println!("cargo:rustc-env=Z3_INCLUDE_DIR={}", z3_include);
    println!("cargo:rustc-env=Z3_SYS_Z3_HEADER={}", z3_h_path.to_str().unwrap());
    println!("cargo:rustc-env=BINDGEN_EXTRA_CLANG_ARGS=-I\"{}\"", z3_include);
    println!("cargo:rustc-env=BINDGEN_EXTRA_CLANG_ARGS_x86_64-pc-windows-msvc=-I\"{}\"", z3_include);
    println!("cargo:rustc-env=BINDGEN_EXTRA_CLANG_ARGS_x86_64_pc_windows_msvc=-I\"{}\"", z3_include);

    // For Windows, we need to ensure libz3.dll is in the PATH
    println!("cargo:rustc-link-search=native={}", z3_bin);
    println!("cargo:rustc-link-lib=dylib=libz3");

    // Tell cargo to rerun if these files change
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed={}\\z3.h", z3_include);
} 