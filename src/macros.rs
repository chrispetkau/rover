use crate::{qmk_name, EXPORT_FOLDER};
use anyhow::Result;
use enum_iterator::{all, Sequence};
use std::{fs, io::Write, path::Path};

#[derive(Debug, PartialEq, Sequence, Clone, Copy)]
pub(crate) enum Macro {
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

pub(crate) fn export_petkau_macros_inl() -> Result<()> {
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
        send_string.push_str(&qmk_name::from_char(c)?);
    }
    send_string.push_str(");");
    Ok(send_string)
}
