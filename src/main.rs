use anyhow::{anyhow, Result};
use guard::*;
use std::{fs, io, path::Path};

// Import folder is hard-coded to "C:\Users\Chris Petkau\Downloads".
const IMPORT_FOLDER: &str = "C:\\Users\\Chris Petkau\\Downloads";

// Export folder is hard-coded to "C:\src\qmk_firmware\keyboards\moonlander\keymaps\chrispetkau".
const EXPORT_FOLDER: &str = "C:\\src\\qmk_firmware\\keyboards\\moonlander\\keymaps\\chrispetkau";

const TEMP_FOLDER: &str = "temp";

struct TempFolder(bool);

impl TempFolder {
    fn new() -> Result<Self> {
        print!("Manifesting '{TEMP_FOLDER}' folder...");
        fs::create_dir(TEMP_FOLDER)?;
        println!("done.");
        Ok(Self(true))
    }

    fn delete(mut self) -> Result<()> {
        print!("Deleting '{TEMP_FOLDER}' folder and all contents...");
        fs::remove_dir_all(TEMP_FOLDER)?;
        println!("done.");
        self.0 = false;
        Ok(())
    }
}

impl Drop for TempFolder {
    fn drop(&mut self) {
        return_unless!(self.0);
        print!("Deleting '{TEMP_FOLDER}' folder and all contents...");
        match fs::remove_dir_all(TEMP_FOLDER) {
            Err(error) => println!("Error deleting '{TEMP_FOLDER}' folder: {error}"),
            Ok(_) => println!("done."),
        }        
    }
}

fn main() -> Result<()> {
    // Find the most recent downloaded file with prefix "moonlander_" and extension ".zip".
    print!("Locating most recent moonlander source code .zip file...");
    let zip = find_zip()?;
    println!("found '{zip}'.");

    let temp = TempFolder::new()?;

    // Extract files and put them in the "temp" folder.
    println!("Unzipping '{zip}' to '{TEMP_FOLDER}' folder...");
    extract_files(&zip)?;
    println!("...done.");

    // Update "config.h" via "temp\config.h": copy every line, then #include "petkau_config.inl".

    // Update "rules.mk" by just overwriting it. There are no customizations to this file.

    update_keymap_c();

    temp.delete()?;

    // Invoke "C:\QMK_MSYS\QMK_MSYS.exe" to run "qmk compile -kb moonlander -km chrispetkau".

    // Stage and commit all changes via git.

    // Invoke "C:\Program Files (x86)\Wally\Wally.exe" to flash.
    // Filename is "C:\src\qmk_firmware\moonlander_chrispetkau.bin".

    Ok(())
}

/// Find the most recent downloaded file with prefix "moonlander_" and extension ".zip".
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
            if file_name.starts_with("moonlander_") && file_name.ends_with(".zip") {
                Some((file_name, time_stamp))
            } else {
                None
            }
        })
        .max_by_key(|(_, time_stamp)| *time_stamp)
        .ok_or_else(|| anyhow!("No .zip file found."))?
        .0)
}

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
                Path::new(TEMP_FOLDER).join(path)
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
