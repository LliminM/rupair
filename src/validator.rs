use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;
use std::fs;

pub struct Validator {
    original_path: PathBuf,
    fixed_path: PathBuf,
}

impl Validator {
    pub fn new(original_path: PathBuf, fixed_path: PathBuf) -> Self {
        Self {
            original_path,
            fixed_path,
        }
    }

    pub fn validate(&self) -> Result<ValidationResult> {
        let original_compiles = self.compile_code(&self.original_path)?;
        let fixed_compiles = self.compile_code(&self.fixed_path)?;

        if !original_compiles || !fixed_compiles {
            return Ok(ValidationResult {
                success: false,
                message: "One or both versions failed to compile".to_string(),
                execution_traces: Vec::new(),
            });
        }

        let original_trace = self.run_and_trace(&self.original_path)?;
        let fixed_trace = self.run_and_trace(&self.fixed_path)?;

        let traces_match = self.compare_traces(&original_trace, &fixed_trace);

        Ok(ValidationResult {
            success: traces_match,
            message: if traces_match {
                "Validation successful: Fixed code maintains semantic equivalence".to_string()
            } else {
                "Validation failed: Fixed code shows different behavior".to_string()
            },
            execution_traces: vec![original_trace, fixed_trace],
        })
    }

    fn compile_code(&self, path: &PathBuf) -> Result<bool> {
        let output = Command::new("rustc")
            .arg(path)
            .output()?;

        Ok(output.status.success())
    }

    fn run_and_trace(&self, path: &PathBuf) -> Result<ExecutionTrace> {
        let output = Command::new(path)
            .output()?;

        Ok(ExecutionTrace {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    fn compare_traces(&self, original: &ExecutionTrace, fixed: &ExecutionTrace) -> bool {
        original.stdout == fixed.stdout && original.exit_code == fixed.exit_code
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub success: bool,
    pub message: String,
    pub execution_traces: Vec<ExecutionTrace>,
}

#[derive(Debug)]
pub struct ExecutionTrace {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub fn validate(fixed_code: &str, original_path: &str) -> Result<()> {
    let temp_dir = tempfile::Builder::new()
        .prefix("rupair_validation")
        .tempdir()?;

    let fixed_path = temp_dir.path().join("fixed.rs");
    fs::write(&fixed_path, fixed_code)?;

    let validator = Validator::new(
        PathBuf::from(original_path),
        fixed_path,
    );

    let result = validator.validate()?;

    if !result.success {
        println!("Validation failed: {}", result.message);
        println!("\nOriginal trace:");
        println!("{}", result.execution_traces[0].stdout);
        println!("\nFixed trace:");
        println!("{}", result.execution_traces[1].stdout);
    } else {
        println!("Validation successful: {}", result.message);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_validation() {
        let temp_dir = tempfile::Builder::new()
            .prefix("rupair_test")
            .tempdir()
            .unwrap();

        let original_path = temp_dir.path().join("original.rs");
        let fixed_path = temp_dir.path().join("fixed.rs");

        fs::write(&original_path, r#"
            fn main() {
                let mut buffer = vec![0u8; 10];
                unsafe {
                    let ptr = buffer.as_mut_ptr();
                    *ptr.add(5) = 42;
                }
                println!("Buffer[5] = {}", buffer[5]);
            }
        "#).unwrap();

        fs::write(&fixed_path, r#"
            fn main() {
                let mut buffer = vec![0u8; 10];
                if 5 < buffer.len() {
                    buffer[5] = 42;
                }
                println!("Buffer[5] = {}", buffer[5]);
            }
        "#).unwrap();

        let validator = Validator::new(original_path, fixed_path);
        let result = validator.validate().unwrap();

        assert!(result.success);
    }
}