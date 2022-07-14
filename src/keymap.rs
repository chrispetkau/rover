use crate::{macros::Macro, qmk_name, temp_folder, EXPORT_FOLDER};
use anyhow::{anyhow, Result};
use enum_iterator::all;
use regex::{Captures, Regex};
use std::{
    fs,
    io::{self, BufRead, Write},
    path::Path,
};

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

pub(crate) fn update_keymap_c() -> Result<()> {
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
    writeln!(keymap_c, "#include \"petkau_tapping_term.inl\"")?;
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
            input_macro.push(qmk_name::to_char(&qmk_name, shifted)?);
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
