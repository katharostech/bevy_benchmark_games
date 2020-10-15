use color_eyre::{Section, SectionExt};
use eyre::{Report, WrapErr};
use tracing as trc;

use std::process::Command;
use std::{path::PathBuf, process::Stdio};

#[trc::instrument]
pub fn build_example(name: &str, headless: bool) -> eyre::Result<String> {
    let mut args = vec!["build", "--release", "--example", name];

    if !headless {
        args.push("--features");
        args.push("with-graphics");
    }

    Ok(Command::new("cargo")
        .args(&args)
        .output_with_err(true)
        .wrap_err("Could not compile example")?)
}

#[trc::instrument]
pub fn run_example(name: &str) -> eyre::Result<String> {
    Ok(
        Command::new(PathBuf::from("./target/release/examples").join(name))
            .output_with_err(false)
            .wrap_err("Could not run example")?,
    )
}

/// Helper trait to get command output and handle errors
trait CommandOutput {
    fn output_with_err(&mut self, inherit_stdout: bool) -> Result<String, Report>;
}

impl CommandOutput for Command {
    #[trc::instrument(level = "debug")]
    fn output_with_err(&mut self, inherit_stdout: bool) -> Result<String, Report> {
        let output = if inherit_stdout {
            self.stderr(Stdio::inherit())
                .stdout(Stdio::inherit())
                .output()?
        } else {
            self.output()?
        };

        let stdout = String::from_utf8_lossy(&output.stdout);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(eyre::format_err!(
                "cmd exited with non-zero status code: {}",
                output
                    .status
                    .code()
                    .map(|x| x.to_string())
                    .unwrap_or("none".to_string())
            ))
            .with_section(move || stdout.trim().to_string().header("Stdout:"))
            .with_section(move || stderr.trim().to_string().header("Stderr:"))
        } else {
            Ok(stdout.into())
        }
    }
}
