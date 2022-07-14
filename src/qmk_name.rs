use anyhow::{anyhow, Result};

/// Maps key names as defined by QKM to chars.
pub(crate) fn to_char(s: &str, shifted: bool) -> Result<char> {
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

pub(crate) fn from_char(c: char) -> Result<String> {
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
