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

    let macros = build_macro_translator(&input_macro_defs)?;

    let custom_keycodes = macros
        .iter()
        .enumerate()
        .filter_map(|(i, petkau_macro)| {
            if petkau_macro.is_none() {
                Some(format!("\tST_MACRO_{i}"))
            } else {
                None
            }
        })
        .collect::<Vec<String>>();
    if !custom_keycodes.is_empty() {
        writeln!(keymap_c)?;
        writeln!(keymap_c, "enum custom_keycodes")?;
        writeln!(keymap_c, "{{")?;
        writeln!(keymap_c, "{}", custom_keycodes.join(",\n"))?;
        writeln!(keymap_c, "}};")?;
    }

    // Write each macro def without a corresponding "petkau" macro.
    // Forward the default case to match the "petkau" macros.
    let cases = Regex::new(
        r"case ST_MACRO_(\d+):[[:space:]]+if \(record->event\.pressed\) \{[[:space:]]+SEND_STRING\(.+\);[[:space:]]+\}[[:space:]]+break;[[:space:]]+",
    )?;
    let macro_defs = cases.replace_all(&input_macro_defs, |captures: &Captures| {
        let i = captures[1].parse::<usize>().unwrap();
        macros[i].map_or(captures[0].to_string(), |_| String::new())
    });
    let macro_defs = Regex::new("(?s:case RGB_SLD:(?:.+)return false;\n)")?.replace(
        &macro_defs,
        "default: return process_record_petkau(keycode, record);\n",
    );
    writeln!(keymap_c)?;
    write!(keymap_c, "{macro_defs}")?;

    // Write the keymap with "petkau" macros installed.
    let keymap = Regex::new(r"ST_MACRO_(\d+)")?.replace_all(&keymap, |captures: &Captures| {
        let i = captures[1].parse::<usize>().unwrap();
        if let Some(petkau_macro) = macros[i] {
            format!("PETKAU_MACRO_{:?}", petkau_macro)
        } else {
            captures[0].to_string()
        }
    });
    write!(keymap_c, "{keymap}")?;

    println!("done.");
    Ok(())
}

/// Map macro indices (i.e. the # in ST_MACRO_#) to the corresponding Macro enum value (which may be None).
fn build_macro_translator(input_macro_defs: &str) -> Result<Vec<Option<Macro>>, anyhow::Error> {
    let tap = "SS_TAP\\(X_([[:alnum:]]+)\\)";
    let shift = "SS_(?:L|R)SFT";
    let shift_tap = &format!("{shift}\\({tap}\\)");
    let control = "SS_(?:L|R)CTL";
    let control_tap = &format!("{control}\\({tap}\\)");
    let taps = Regex::new(tap)?;
    let control_taps = Regex::new(control_tap)?;
    let shift_taps = Regex::new(shift_tap)?;
    let all_taps = Regex::new(&format!("{control_tap}|{shift_tap}|{tap}"))?;
    Ok(Regex::new(r"SEND_STRING\((.+)\);\n")?
        .captures_iter(input_macro_defs)
        .map(|send_string| {
            all_taps
                .captures_iter(&send_string[1])
                .map(|tap| {
                    let full_text = &tap[0];
                    if control_taps.is_match(full_text) {
                        return Err(anyhow!("Macro uses Ctrl."));
                    }
                    let shifted = shift_taps.is_match(full_text);
                    let qmk_name = if shifted {
                        shift_taps.captures(full_text).unwrap()[1].to_string()
                    } else {
                        taps.captures(full_text).unwrap()[1].to_string()
                    };
                    qmk_name::to_char(&qmk_name, shifted)
                })
                .collect::<Result<String>>()
                .map_or(None, |macro_prefix| Some(macro_prefix.to_ascii_lowercase()))
                .and_then(|macro_prefix| {
                    let mut matching_macros =
                        all::<Macro>().filter(|&value| String::from(value).starts_with(&macro_prefix));
                    match matching_macros.clone().count() {
                        0 => {
                            println!("No macro matches '{macro_prefix}'. Using it literally.");
                            None
                        }
                        1 => Some(matching_macros.next().unwrap()),
                        _ => {
                            let first = matching_macros.next().unwrap();
                            println!(
                                "Multiple macro matches for '{macro_prefix}': {:?}. Using the first match '{}'.",
                                matching_macros.collect::<Vec<_>>(),String::from(first)
                            );
                            Some(first)
                        }
                    }
            })
        })
        .collect::<Vec<_>>())
}
