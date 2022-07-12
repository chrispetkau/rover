use crate::temp_folder::TempFolder;
use anyhow::{anyhow, Result};
use enum_iterator::{all, Sequence};
use guard::*;
use regex::{Captures, Regex};
use std::{
    fs,
    io::{self, BufRead, Write},
    path::Path,
    process::Command,
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
            print!("\nDropping '{NAME}' folder and all contents...");
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
    io::copy(
        &mut fs::File::open(Path::new(temp_folder::NAME).join("rules.mk"))?,
        &mut fs::File::create(Path::new(EXPORT_FOLDER).join("rules.mk"))?,
    )?;
    println!("done.");

    update_keymap_c()?;

    export_petkau_macros_inl()?;

    temp_folder.delete()?;

    // Invoke "C:\QMK_MSYS\QMK_MSYS.exe" to run "qmk compile -kb moonlander -km chrispetkau".
    print!("Compiling QMK firmware...");
    // > C:/QMK_MSYS/conemu/ConEmu64.exe -NoSingle -NoUpdate -icon "C:/QMK_MSYS/icon.ico" -title "QMK MSYS" -run "C:/QMK_MSYS/usr/bin/bash.exe" -l -t -c "qmk compile -kb moonlander -km chrispetkau"
    let output = Command::new("C:/QMK_MSYS/conemu/ConEmu64.exe")
        .args([
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
            "qmk compile -kb moonlander -km chrispetkau"
        ])
        .output()?;
    if output.status.success() {
        println!("done.");
    } else {
        println!("failed.");
    }

    // Invoke "C:\Program Files (x86)\Wally\Wally.exe" to flash.
    // Filename is "C:\src\qmk_firmware\moonlander_chrispetkau.bin".
    print!("Flashing keyboard...");
    let output = Command::new("C:/Program Files (x86)/Wally/Wally.exe")
        .args(["C:/src/qmk_firmware/moonlander_chrispetkau.bin"])
        .output()?;
    if output.status.success() {
        println!("done.");
    } else {
        println!("failed.");
    }

    // Stage and commit all changes via git.
    print!("Committing changes...");
    let output = Command::new("git")
        .current_dir(EXPORT_FOLDER)
        .args(["commit", "-am", &zip])
        .output()?;
    if output.status.success() {
        println!("done.");
    } else {
        println!("failed.");
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

enum KeymapSection {
    Prepocessing,
    MacroEnum,
    TapDanceEnum,
    Keymap,
    RGBSetup,
    MacroDefs,
    TapDanceSetup,
    TapDanceDefs,
}

fn update_keymap_c() -> Result<()> {
    print!("Updating keymap.c...");
    let input = &mut fs::File::open(Path::new(temp_folder::NAME).join("keymap.c"))?;
    let keymap_c = &mut fs::File::create(Path::new(EXPORT_FOLDER).join("keymap.c"))?;
    let petkau_tap_dance_inl =
        &mut fs::File::create(Path::new(EXPORT_FOLDER).join("petkau_tap_dance.inl"))?;

    // keymap.c has the following format:
    // - preprocessing
    // - macro_enum
    // - tap_dance_enum
    // - keymap
    // - rgb_setup
    // - macro_defs
    // - tap_dance_setup
    // - tap_dance_defs
    //
    // The new keymap.c exports like this:
    // - preprocessing
    // - #include "petkau_macros.inl"
    // - tap_dance_enum
    // - rgb_setup
    // - tap_dance_setup
    // - #include "petkau_tap_dance.inl"
    // - custom_keymap
    let mut input_section = KeymapSection::Prepocessing;
    let mut input_macro_defs = String::new();
    let mut keymap = String::new();
    let mut macro_count = 0;
    for line in io::BufReader::new(input).lines() {
        let line = line?;
        match input_section {
            KeymapSection::Prepocessing => {
                if line == "enum custom_keycodes {" {
                    writeln!(keymap_c, "#include \"petkau_macros.inl\"")?;
                    input_section = KeymapSection::MacroEnum;
                } else {
                    writeln!(keymap_c, "{line}")?;
                }
            }
            KeymapSection::MacroEnum => {
                if line == "enum tap_dance_codes {" {
                    writeln!(petkau_tap_dance_inl, "{line}")?;
                    input_section = KeymapSection::TapDanceEnum;
                } else if line != "};\n" {
                    macro_count += 1;
                }
            }
            KeymapSection::TapDanceEnum => {
                if line == "const uint16_t PROGMEM keymaps[][MATRIX_ROWS][MATRIX_COLS] = {" {
                    keymap.push_str(&line);
                    keymap.push('\n');
                    input_section = KeymapSection::Keymap;
                } else {
                    writeln!(petkau_tap_dance_inl, "{line}")?;
                }
            }
            KeymapSection::Keymap => {
                keymap.push_str(&line);
                keymap.push('\n');
                if line == "extern rgb_config_t rgb_matrix_config;" {
                    input_section = KeymapSection::RGBSetup;
                }
            }
            KeymapSection::RGBSetup => {
                if line == "bool process_record_user(uint16_t keycode, keyrecord_t *record) {" {
                    input_macro_defs.push_str(&line);
                    input_macro_defs.push('\n');
                    input_section = KeymapSection::MacroDefs;
                } else {
                    writeln!(keymap_c, "{line}")?;
                }
            }
            KeymapSection::MacroDefs => {
                if line == "typedef struct {" {
                    writeln!(keymap_c, "{line}")?;
                    input_section = KeymapSection::TapDanceSetup;
                } else {
                    input_macro_defs.push_str(&line);
                    input_macro_defs.push('\n');
                }
            }
            KeymapSection::TapDanceSetup => {
                if line.starts_with("static tap dance_state") {
                    writeln!(petkau_tap_dance_inl, "{line}")?;
                    input_section = KeymapSection::TapDanceDefs;
                } else {
                    writeln!(keymap_c, "{line}")?;
                }
            }
            // Overwrite petkau_tap_dance.inl with tap_dance_defs.
            KeymapSection::TapDanceDefs => writeln!(petkau_tap_dance_inl, "{line}")?,
        }
    }
    writeln!(keymap_c, "#include \"petkau_tap_dance.inl\"")?;

    // Map macros by parsing the arguments of the SEND_STRING calls in macro_defs and converting them into strings.
    // These strings are then matched against the strings known to correspond to hard-coded macros. Strings
    // extracted from the macro_defs may not be complete and so partial matches must be considered. Multiple matches
    // may occur and constitute an error: strings in macro_defs must match only a single macro in petkau_macros.inl.
    // This produces an array whose indices map to ST_MACRO_# and contents map to the corresponding enum value of the
    // hard-coded macro enum in petkau_macros.inl.
    let mut macro_prefixes: Vec<String> = Vec::with_capacity(macro_count);
    let send_strings = Regex::new(r"SEND_STRING\((.+)\);\n")?;
    let ss_taps = Regex::new(r"SS_TAP\(X_([[:alnum:]]+)\)")?;
    let shift_taps = Regex::new(r"(?:SS_LSFT|SS_RSFT)\(SS_TAP\(X_([[:alnum:]]+)\)\)")?;
    let shift_or_ss_taps = Regex::new(
        r"((?:SS_LSFT|SS_RSFT)\(SS_TAP\(X_(?:[[:alnum:]]+)\)\)|SS_TAP\(X_(?:[[:alnum:]]+)\))",
    )?;
    for send_string in send_strings.captures_iter(&input_macro_defs) {
        let mut input_macro = String::new();
        let send_string = &send_string[1];
        for shift_or_ss_tap in shift_or_ss_taps.captures_iter(send_string) {
            let full_text = &shift_or_ss_tap[1];
            let shifted = shift_taps.is_match(full_text);
            let qmk_name = if shifted {
                shift_taps.captures(full_text).unwrap()[1].to_string()
            } else {
                ss_taps.captures(full_text).unwrap()[1].to_string()
            };
            input_macro.push(char_from_qmk_name(&qmk_name, shifted)?);
        }
        macro_prefixes.push(input_macro.to_ascii_lowercase());
    }

    // Map macro_prefixes to macros.
    let mut macros: Vec<Macro> = Vec::with_capacity(macro_count);
    for macro_prefix in &macro_prefixes {
        let mut matching_macros =
            all::<Macro>().filter(|&value| String::from(value).starts_with(macro_prefix));
        match matching_macros.clone().count() {
            0 => return Err(anyhow!("No macro matches '{macro_prefix}'")),
            1 => macros.push(matching_macros.next().unwrap()),
            _ => {
                return Err(anyhow!(
                    "Multiple macro matches for '{macro_prefix}': {:?}",
                    matching_macros.collect::<Vec<_>>()
                ))
            }
        };
    }

    // Create custom_keymap by replacing all occurrences of "ST_MACRO_#" in keymap with the corresponding entry in the
    // macro map.
    let keymap = Regex::new(r"ST_MACRO_(\d+)")?.replace_all(&keymap, |captures: &Captures| {
        format!(
            "PETKAU_MACRO_{:?}",
            macros[captures[1].parse::<usize>().unwrap()]
        )
    });

    writeln!(keymap_c)?;
    write!(keymap_c, "{keymap}")?;

    println!("done.");
    Ok(())
}

/// Maps key names as defined by QKM to chars.
fn char_from_qmk_name(s: &str, shifted: bool) -> Result<char> {
    if s.len() == 1 {
        let c = s
            .chars()
            .next()
            .ok_or_else(|| anyhow!("{s} has no contents."))?;
        return match c {
            'A'..='Z' => Ok(if shifted {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            }),
            '0'..='9' => Ok(match c {
                '1' => '!',
                '2' => '@',
                '3' => '#',
                '4' => '$',
                '5' => '%',
                '6' => '&',
                '7' => '&',
                '8' => '*',
                '9' => '(',
                '0' => ')',
                _ => c,
            }),
            '=' => Ok(if shifted { '+' } else { c }),
            '-' => Ok(if shifted { '_' } else { c }),
            '.' => Ok(if shifted { '>' } else { c }),
            ',' => Ok(if shifted { '<' } else { c }),
            _ => Err(anyhow!("No known QMK name for {c}.")),
        };
    }
    match s {
        "EQUAL" => Ok(if shifted { '+' } else { '=' }),
        "MINUS" => Ok(if shifted { '_' } else { '-' }),
        "DOT" => Ok(if shifted { '>' } else { '.' }),
        "COMMA" => Ok(if shifted { '<' } else { ',' }),
        _ => Err(anyhow!("No known QMK name for {s}.")),
    }
}

fn char_to_qmk_name(c: char) -> Result<String> {
    match c {
        'a'..='z' | '0'..='9' => Ok(format!("SS_TAP(X_{})", c.to_ascii_uppercase())),
        'A'..='Z' => Ok(format!("SS_LSFT(SS_TAP({c}))")),
        '=' => Ok("SS_TAP(X_EQUAL)".to_string()),
        '+' => Ok("SS_LSFT(SS_TAP(X_EQUAL))".to_string()),
        '-' => Ok("SS_TAP(X_MINUS)".to_string()),
        '_' => Ok("SS_LSFT(SS_TAP(X_MINUS))".to_string()),
        '.' => Ok("SS_TAP(X_DOT)".to_string()),
        '!' => Ok("SS_LSFT(SS_TAP(X_1))".to_string()),
        '@' => Ok("SS_LSFT(SS_TAP(X_2))".to_string()),
        '#' => Ok("SS_LSFT(SS_TAP(X_3))".to_string()),
        '$' => Ok("SS_LSFT(SS_TAP(X_4))".to_string()),
        '%' => Ok("SS_LSFT(SS_TAP(X_5))".to_string()),
        '^' => Ok("SS_LSFT(SS_TAP(X_6))".to_string()),
        '&' => Ok("SS_LSFT(SS_TAP(X_7))".to_string()),
        '*' => Ok("SS_LSFT(SS_TAP(X_8))".to_string()),
        '(' => Ok("SS_LSFT(SS_TAP(X_9))".to_string()),
        ')' => Ok("SS_LSFT(SS_TAP(X_0))".to_string()),
        '<' => Ok("SS_LSFT(SS_TAP(X_COMMA))".to_string()),
        '>' => Ok("SS_LSFT(SS_TAP(X_DOT))".to_string()),
        _ => Err(anyhow!("No known QMK name for {c}.")),
    }
}

#[derive(Debug, PartialEq, Sequence, Clone, Copy)]
enum Macro {
    Void,
    Break,
    NotEqual,
    EqualsArrow,
    DashArrow,
    Return,
    Bool,
    False,
    True,
    NullPtr,
    Continue,
    Virtual,
    Override,
    Static,
    Enum,
    Class,
    Struct,
    Namespace,
    Include,
    Define,
    IfDef,
    Else,
    EndIf,
    Public,
    Private,
    Template,
    Typename,
    Auto,
    While,
    ReinterpretCast,
}

impl From<Macro> for String {
    fn from(m: Macro) -> Self {
        match m {
            Macro::Void => "void",
            Macro::Break => "break",
            Macro::NotEqual => "!=",
            Macro::EqualsArrow => "=>",
            Macro::DashArrow => "->",
            Macro::Return => "return",
            Macro::Bool => "bool",
            Macro::False => "false",
            Macro::True => "true",
            Macro::NullPtr => "nullptr",
            Macro::Continue => "continue",
            Macro::Virtual => "virtual",
            Macro::Override => "override",
            Macro::Static => "static",
            Macro::Enum => "enum",
            Macro::Class => "class",
            Macro::Struct => "struct",
            Macro::Namespace => "namespace",
            Macro::Include => "#include",
            Macro::Define => "#define",
            Macro::IfDef => "#ifdef",
            Macro::Else => "#else",
            Macro::EndIf => "#endif",
            Macro::Public => "public",
            Macro::Private => "private",
            Macro::Template => "template",
            Macro::Typename => "typename",
            Macro::Auto => "auto",
            Macro::While => "while",
            Macro::ReinterpretCast => "reinterpret_cast",
        }
        .to_string()
    }
}

fn export_petkau_macros_inl() -> Result<()> {
    print!("Exporting petkau_macros.inl...");
    let petkau_macros_inl =
        &mut fs::File::create(Path::new(EXPORT_FOLDER).join("petkau_macros.inl"))?;
    writeln!(petkau_macros_inl, "enum custom_keycodes")?;
    writeln!(petkau_macros_inl, "{{")?;
    writeln!(petkau_macros_inl, "\tRGB_SLD = ML_SAFE_RANGE,")?;
    for value in all::<Macro>() {
        writeln!(petkau_macros_inl, "\tPETKAU_MACRO_{:?},", value)?;
    }
    writeln!(petkau_macros_inl, "}};")?;
    writeln!(petkau_macros_inl)?;
    writeln!(petkau_macros_inl, "#define PETKAU_DELAY SS_DELAY(0)")?;
    writeln!(petkau_macros_inl)?;
    writeln!(
        petkau_macros_inl,
        "bool process_record_user(uint16_t keycode, keyrecord_t *record)"
    )?;
    writeln!(petkau_macros_inl, "{{")?;
    writeln!(petkau_macros_inl, "\tswitch (keycode)")?;
    writeln!(petkau_macros_inl, "\t{{")?;
    for value in all::<Macro>() {
        writeln!(petkau_macros_inl, "\tcase PETKAU_MACRO_{:?}:", value)?;
        writeln!(petkau_macros_inl, "\t\tif (record->event.pressed)")?;
        writeln!(petkau_macros_inl, "\t\t\t{}", make_send_string(value)?)?;
        writeln!(petkau_macros_inl, "\t\tbreak;")?;
    }
    writeln!(petkau_macros_inl, "\tcase RGB_SLD:")?;
    writeln!(
        petkau_macros_inl,
        "\t\tif (record->event.pressed) rgblight_mode(1);"
    )?;
    writeln!(petkau_macros_inl, "\t\treturn false;")?;
    writeln!(petkau_macros_inl, "\t}}")?;
    writeln!(petkau_macros_inl, "\treturn true;")?;
    writeln!(petkau_macros_inl, "}};")?;
    println!("done.");
    Ok(())
}

fn make_send_string(value: Macro) -> Result<String> {
    let value = String::from(value);
    let mut send_string = "SEND_STRING(".to_string();
    let mut first = true;
    for c in value.chars() {
        if first {
            first = false;
        } else {
            send_string.push_str(" PETKAU_DELAY ");
        }
        send_string.push_str(&char_to_qmk_name(c)?);
    }
    send_string.push_str(");");
    Ok(send_string)
}
