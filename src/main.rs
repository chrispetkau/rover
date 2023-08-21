use crate::temp_folder::TempFolder;
use anyhow::Result;
use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::Command,
};

// Export folder is hard-coded to "C:\src\qmk_firmware\keyboards\moonlander\keymaps\chrispetkau".
const EXPORT_FOLDER: &str = "C:/src/qmk_firmware/keyboards/moonlander/keymaps/chrispetkau";

mod command;
mod keymap;
mod macros;
mod qmk_name;
mod temp_folder;
mod zip;
mod custom_keycode;

fn main() -> Result<()> {
    // Find the most recent downloaded file with prefix "moonlander_" and extension ".zip".
    print!("Locating most recent moonlander_* source code .zip file...");
    let zip = zip::find_most_recent_download()?;
    println!("found '{zip}'.");

    let temp_folder = TempFolder::new()?;

    println!("Unzipping '{zip}' to '{}' folder...", temp_folder::NAME);
    zip::extract_files_to_temp(&zip)?;
    println!("...done.");

    // Update "config.h" via "temp\config.h": copy every line, then #include "petkau_config.inl".
    print!("Updating config.h...");
    let config = &mut fs::File::create(Path::new(EXPORT_FOLDER).join("config.h"))?;
    io::copy(
        &mut fs::File::open(Path::new(temp_folder::NAME).join("config.h"))?,
        config,
    )?;
    writeln!(config, "#include \"petkau_config.inl\"")?;
    println!("done.");

    // Update "rules.mk" by just overwriting it. There are no customizations to this file.
    print!("Updating rules.mk...");
    let rules = &mut fs::File::create(Path::new(EXPORT_FOLDER).join("rules.mk"))?;
    io::copy(
        &mut fs::File::open(Path::new(temp_folder::NAME).join("rules.mk"))?,
        rules,
    )?;
    writeln!(rules, "DYNAMIC_TAPPING_TERM_ENABLE = yes")?;
    println!("done.");

    keymap::update_keymap_c()?;

    temp_folder.delete()?;

    macros::export_petkau_macros_inl()?;

    // > C:/QMK_MSYS/conemu/ConEmu64.exe -NoSingle -NoUpdate -icon "C:/QMK_MSYS/icon.ico" -title "QMK MSYS" -run "C:/QMK_MSYS/usr/bin/bash.exe" -l -t -c "qmk compile -kb moonlander -km chrispetkau"
    command::run(
        "Compiling QMK firmware",
        Command::new("C:/QMK_MSYS/conemu/ConEmu64.exe").args([
            "-NoSingle",
            "-NoUpdate",
            "-icon",
            "C:/QMK_MSYS/icon.ico",
            "-title",
            "QMK MSYS",
            "-run",
            "C:/QMK_MSYS/usr/bin/bash.exe",
            "-l",
            "-t",
            "-c",
            "qmk compile -j 0 -kb moonlander -km chrispetkau",
        ]),
    )?;

    command::run(
        "Flashing keyboard",
        Command::new("C:/Program Files (x86)/Wally/Wally.exe")
            .args(["C:/src/qmk_firmware/moonlander_chrispetkau.bin"]),
    )?;

    // Stage and commit all changes via git.
    command::run(
        "Committing changes",
        Command::new("git")
            .current_dir(EXPORT_FOLDER)
            .args(["commit", "-am", &zip]),
    )?;

    Ok(())
}
