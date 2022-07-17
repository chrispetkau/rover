use anyhow::{anyhow, Result};
use std::process::Command;

pub(crate) fn run(title: &str, command: &mut Command) -> Result<()> {
    print!("{title}...");
    if command.output()?.status.success() {
        println!("done.");
        Ok(())
    } else {
        println!("failed.");
        Err(anyhow!("Command failed: {:#?}", command))
    }
}
