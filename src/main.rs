use crate::temp_folder::TempFolder;
use anyhow::{anyhow, Result};
use guard::*;
use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::{Command, ExitStatus},
};

// Import folder is hard-coded to "C:\Users\Chris Petkau\Downloads".
const IMPORT_FOLDER: &str = "C:/Users/Chris Petkau/Downloads";

// Export folder is hard-coded to "C:\src\qmk_firmware\keyboards\moonlander\keymaps\chrispetkau".
const EXPORT_FOLDER: &str = "C:/src/qmk_firmware/keyboards/moonlander/keymaps/chrispetkau";

mod temp_folder {
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
            print!("Deleting '{NAME}' folder and all contents...");
            match fs::remove_dir_all(NAME) {
                Err(error) => println!("Error deleting '{NAME}' folder: {error}"),
                Ok(_) => println!("done."),
            }
        }
    }
}

fn main() -> Result<()> {
    // Find the most recent downloaded file with prefix "moonlander_" and extension ".zip".
    print!("Locating most recent moonlander_colemak_coder source code .zip file...");
    let zip = find_zip()?;
    println!("found '{zip}'.");

    let temp_folder = TempFolder::new()?;

    println!("Unzipping '{zip}' to '{}' folder...", temp_folder::NAME);
    extract_files(&zip)?;
    println!("...done.");

    // Update "config.h" via "temp\config.h": copy every line, then #include "petkau_config.inl".
    println!("Updating config.h...");
    let config = &mut fs::File::create(Path::new(EXPORT_FOLDER).join("config.h"))?;
    io::copy(
        &mut fs::File::open(Path::new(temp_folder::NAME).join("config.h"))?,
        config,
    )?;
    writeln!(config, "#include \"petkau_config.inl\"")?;
    println!("...done.");

    // Update "rules.mk" by just overwriting it. There are no customizations to this file.
    println!("Updating rules.mk...");
    io::copy(
        &mut fs::File::open(Path::new(temp_folder::NAME).join("rules.mk"))?,
        &mut fs::File::create(Path::new(EXPORT_FOLDER).join("rules.mk"))?,
    )?;
    println!("...done.");

    update_keymap_c();

    temp_folder.delete()?;

    // Invoke "C:\QMK_MSYS\QMK_MSYS.exe" to run "qmk compile -kb moonlander -km chrispetkau".
    // println!("Compiling QMK firmware...");
    // let output = Command::new("C:/QMK_MSYS/conemu/ConEmu64.exe")
    //     .current_dir("C:/QMK_MSYS/conemu")
    //     .args([
    //         "-NoSingle",
    //         "-NoUpdate",
    //         "-icon",
    //         "C:/QMK_MSYS/icon.ico",
    //         "-title",
    //         "QMK MSYS",
    //         "-run",
    //         "C:/QMK_MSYS/usr/bin/bash.exe",
    //         "-l",
    //         "-i",
    //         //"-t",
    //         // "-c",
    //         // "\"qmk compile -kb moonlander -km chrispetkau\"",
    //         //"-cur_console:m:\"\"",
    //     ])
    //     .output()?;
    // if output.status.success() {
    //     println!("...done.");
    // } else {
    //     println!("...failed.");
    //     println!("StdOut:");
    //     println!("{:?}", output.stdout);
    //     println!("StdErr:");
    //     println!("{:?}", output.stderr);
    // }

    // Invoke "C:\Program Files (x86)\Wally\Wally.exe" to flash.
    // Filename is "C:\src\qmk_firmware\moonlander_chrispetkau.bin".
    // println!("Flashing keyboard...");
    // let output = Command::new("C:/Program Files (x86)/Wally/Wally.exe")
    //     .args(["C:/src/qmk_firmware/moonlander_chrispetkau.bin"])
    //     .output()?;
    // if output.status.success() {
    //     println!("...done.");
    // } else {
    //     println!("...failed.");
    //     println!("StdOut:");
    //     println!("{:?}", output.stdout);
    //     println!("StdErr:");
    //     println!("{:?}", output.stderr);
    // }

    // Stage and commit all changes via git.
    println!("Committing changes...");
    let output = Command::new("git")
        .current_dir(EXPORT_FOLDER)
        .args(["commit", "-am", &zip])
        .output()?;
    if output.status.success() {
        println!("...done.");
    } else {
        println!("...failed.");
        println!("StdOut:");
        println!("{:?}", output.stdout);
        println!("StdErr:");
        println!("{:?}", output.stderr);
    }

    Ok(())
}

/// Find the most recent downloaded file with prefix "moonlander_colemak_coder_" and extension ".zip".
fn find_zip() -> Result<String> {
    Ok(fs::read_dir(IMPORT_FOLDER)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_name = entry.file_name().into_string().ok()?;
            let time_stamp = entry.metadata().ok()?.modified().ok()?;
            if entry.file_type().ok()?.is_file() {
                Some((file_name, time_stamp))
            } else {
                None
            }
        })
        .filter_map(|(file_name, time_stamp)| {
            if file_name.starts_with("moonlander_colemak_coder_") && file_name.ends_with(".zip") {
                Some((file_name, time_stamp))
            } else {
                None
            }
        })
        .max_by_key(|(_, time_stamp)| *time_stamp)
        .ok_or_else(|| anyhow!("No .zip file found."))?
        .0)
}

/// Extract files and put them in the "temp" folder.
fn extract_files(zip: &str) -> Result<()> {
    let mut zip = zip::ZipArchive::new(fs::File::open(Path::new(IMPORT_FOLDER).join(&zip))?)?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let file_name = file.name();
        continue_unless!(file_name.starts_with("moonlander_colemak_coder_source"));
        continue_unless!(!file_name.ends_with('/'));
        let outpath = match file.enclosed_name() {
            Some(path) => {
                let path = Path::new(
                    path.components()
                        .last()
                        .ok_or_else(|| anyhow!("Empty filename in .zip file."))?
                        .as_os_str(),
                );
                Path::new(temp_folder::NAME).join(path)
            }
            None => continue,
        };
        println!(
            "Entry {} is a file. Extracting \"{}\" ({} bytes)",
            i,
            outpath.display(),
            file.size()
        );
        io::copy(&mut file, &mut fs::File::create(&outpath)?)?;
    }
    Ok(())
}

fn update_keymap_c() {
    // keymap.c has the following format:
    // - preprocessing
    // - macro_enum
    // - tap_dance_enum
    // - keymap
    // - rgb_setup
    // - macro_defs
    // - tap_dance_setup
    // - tap_dance_defs

    // Overwrite petkau_tap_dance.inl with tap_dance_defs.

    // Map macros by parsing the arguments of the SEND_STRING calls in macro_defs and converting them into strings.
    // These strings are then matched against the strings known to correspond to hard-coded macros. Strings
    // extracted from the macro_defs may not be complete and so partial matches must be considered. Multiple matches
    // may occur and constitute an error: strings in macro_defs must match only a single macro in petkau_macros.inl.
    // This produces an array whose indices map to ST_MACRO_# and contents map to the corresponding enum value of the
    // hard-coded macro enum in petkau_macros.inl.

    // Create custom_keymap by replacing all occurrences of "ST_MACRO_#" in keymap with the corresponding entry in the
    // macro map.

    // The new keymap.c exports like this:
    // - preprocessing
    // - #include "petkau_macros.inl"
    // - tap_dance_enum
    // - custom_keymap
    // - rgb_setup
    // - tap_dance_setup
    // - #include "petkau_tap_dance.inl"
}
