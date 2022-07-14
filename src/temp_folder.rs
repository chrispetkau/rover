use anyhow::Result;
use guard::*;
use std::fs;

pub(crate) const NAME: &str = "temp";

pub(crate) struct TempFolder(bool);

impl TempFolder {
    pub(crate) fn new() -> Result<Self> {
        print!("Manifesting '{NAME}' folder...");
        fs::create_dir(NAME)?;
        println!("done.");
        Ok(Self(true))
    }

    pub(crate) fn delete(mut self) -> Result<()> {
        print!("Deleting '{NAME}' folder and all contents...");
        fs::remove_dir_all(NAME)?;
        println!("done.");
        self.0 = false;
        Ok(())
    }
}

impl Drop for TempFolder {
    fn drop(&mut self) {
        return_unless!(self.0);
        print!("\nDropping '{NAME}' folder and all contents...");
        match fs::remove_dir_all(NAME) {
            Err(error) => println!("Error deleting '{NAME}' folder: {error}"),
            Ok(_) => println!("done."),
        }
    }
}
