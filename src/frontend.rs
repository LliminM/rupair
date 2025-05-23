use syn::{File, parse_file};
use anyhow::Result;
use std::fs;

pub fn parse_rust_file(path: &str) -> Result<File> {
    let src = fs::read_to_string(path)?;
    let ast = parse_file(&src)?;
    Ok(ast)
}