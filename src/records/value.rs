//! Converts between JSON values and SQLite values, by field type.

use std::collections::HashSet;

use rusqlite::types::Value as Sql;
use serde_json::Value;

use crate::{catalog::FieldOut, error::AppError};

const MAX_TEXT: usize = 100_000;
const MAX_LINKS: usize = 1000;

/// Validates `v` for `field` and returns the value to store. JSON `null` clears the field.
pub fn to_sql(field: &FieldOut, v: &Value) -> Result<Sql, AppError> {
    if v.is_null() {
        return Ok(Sql::Null);
    }
    let bad = |msg: &str| AppError::BadRequest(format!("field `{}`: {msg}", field.name));
    match field.field_type {
        "text" => match v.as_str() {
            Some(s) if s.chars().count() <= MAX_TEXT => Ok(Sql::Text(s.to_string())),
            Some(_) => Err(bad(&format!("text is longer than {MAX_TEXT} characters"))),
            None => Err(bad("expected a string")),
        },
        "phone" => {
            let s = v.as_str().ok_or_else(|| bad("expected a string"))?;
            normalize_phone(s).map(Sql::Text).map_err(|m| bad(&m))
        }
        "date" => {
            let s = v.as_str().ok_or_else(|| bad("expected a string"))?;
            if valid_date(s) {
                Ok(Sql::Text(s.to_string()))
            } else {
                Err(bad("expected an ISO 8601 date (2026-03-01) or date-time (2026-03-01T09:30:00Z)"))
            }
        }
        "number" => {
            let Value::Number(n) = v else { return Err(bad("expected a number")) };
            if field.config.get("integer").and_then(Value::as_bool) == Some(true) {
                let whole = n.as_i64().or_else(|| {
                    n.as_f64().filter(|f| f.fract() == 0.0 && f.abs() < 9.0e18).map(|f| f as i64)
                });
                whole.map(Sql::Integer).ok_or_else(|| bad("expected a whole number"))
            } else {
                n.as_f64().map(Sql::Real).ok_or_else(|| bad("expected a number"))
            }
        }
        "boolean" => v.as_bool().map(|b| Sql::Integer(b as i64)).ok_or_else(|| bad("expected true or false")),
        "select" => {
            let s = v.as_str().ok_or_else(|| bad("expected an option label or id"))?;
            option_id(field, s)
                .map(Sql::Text)
                .ok_or_else(|| bad(&format!("`{s}` is not one of this field's options")))
        }
        // A to-one relation holds one record ID. Whether that record exists is checked against the database.
        "relation" if !field.many_relation() => match v.as_str() {
            Some(id) if !id.is_empty() => Ok(Sql::Text(id.to_string())),
            _ => Err(bad("expected a record id")),
        },
        other => Err(AppError::Internal(format!("field type `{other}` is not stored in a column"))),
    }
}

/// Resolves an option label (case-insensitive) or ID to the option's ID.
pub fn option_id(field: &FieldOut, label_or_id: &str) -> Option<String> {
    field
        .options
        .iter()
        .find(|o| o.id == label_or_id)
        .or_else(|| field.options.iter().find(|o| o.label.to_lowercase() == label_or_id.to_lowercase()))
        .map(|o| o.id.clone())
}

/// Validates the value of a field stored in a join table, and returns the IDs to link, in order.
/// Multi-select takes option labels or IDs; a to-many relation takes record IDs. `null` means none.
pub fn to_links(field: &FieldOut, v: &Value) -> Result<Vec<String>, AppError> {
    let bad = |msg: &str| AppError::BadRequest(format!("field `{}`: {msg}", field.name));
    if v.is_null() {
        return Ok(Vec::new());
    }
    let items = v.as_array().ok_or_else(|| bad("expected an array"))?;
    if items.len() > MAX_LINKS {
        return Err(bad(&format!("at most {MAX_LINKS} values")));
    }
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let s = item.as_str().filter(|s| !s.is_empty()).ok_or_else(|| bad("expected an array of strings"))?;
        let id = if field.field_type == "multi_select" {
            option_id(field, s).ok_or_else(|| bad(&format!("`{s}` is not one of this field's options")))?
        } else {
            s.to_string()
        };
        if !seen.insert(id.clone()) {
            return Err(bad(&format!("`{s}` is listed more than once")));
        }
        out.push(id);
    }
    Ok(out)
}

/// Converts a stored value to JSON. Select values come out as the option label.
pub fn from_sql(field: &FieldOut, v: Sql) -> Value {
    match (field.field_type, v) {
        (_, Sql::Null) | (_, Sql::Blob(_)) => Value::Null,
        ("boolean", Sql::Integer(i)) => Value::Bool(i != 0),
        ("select", Sql::Text(id)) => field
            .options
            .iter()
            .find(|o| o.id == id)
            .map_or(Value::Null, |o| Value::String(o.label.clone())),
        (_, Sql::Integer(i)) => Value::from(i),
        (_, Sql::Real(f)) => serde_json::Number::from_f64(f).map_or(Value::Null, Value::Number),
        (_, Sql::Text(s)) => Value::String(s),
    }
}

/// Strips spaces, dashes, dots and parentheses, and requires E.164: `+` and 7 to 15 digits, not starting with 0.
pub fn normalize_phone(raw: &str) -> Result<String, String> {
    let cleaned: String = raw.chars().filter(|c| !matches!(c, ' ' | '-' | '.' | '(' | ')')).collect();
    let digits = cleaned.strip_prefix('+').unwrap_or("");
    let ok = (7..=15).contains(&digits.len())
        && digits.bytes().all(|b| b.is_ascii_digit())
        && !digits.starts_with('0');
    if ok {
        Ok(cleaned)
    } else {
        Err("expected an international number with country code, like +14155552671".into())
    }
}

/// Accepts `YYYY-MM-DD`, or a date-time `YYYY-MM-DDTHH:MM:SS[.fraction]` followed by `Z` or `±HH:MM`.
pub fn valid_date(s: &str) -> bool {
    if !s.is_ascii() || s.len() < 10 {
        return false;
    }
    let b = s.as_bytes();
    let num = |from: usize, to: usize| -> Option<u32> {
        let part = &s[from..to];
        if part.bytes().all(|c| c.is_ascii_digit()) { part.parse().ok() } else { None }
    };
    if b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let (Some(y), Some(m), Some(d)) = (num(0, 4), num(5, 7), num(8, 10)) else { return false };
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    if d < 1 || d > days {
        return false;
    }
    if s.len() == 10 {
        return true;
    }

    // Time part, starting at the `T`.
    if b[10] != b'T' || s.len() < 20 || b[13] != b':' || b[16] != b':' {
        return false;
    }
    let (Some(hh), Some(mm), Some(ss)) = (num(11, 13), num(14, 16), num(17, 19)) else { return false };
    if hh > 23 || mm > 59 || ss > 59 {
        return false;
    }
    let mut tail = &s[19..];
    if let Some(frac) = tail.strip_prefix('.') {
        let n = frac.bytes().take_while(u8::is_ascii_digit).count();
        if n == 0 {
            return false;
        }
        tail = &frac[n..];
    }
    match tail.as_bytes() {
        [b'Z'] => true,
        [b'+' | b'-', h1, h2, b':', m1, m2] => {
            let digits = [h1, h2, m1, m2];
            digits.iter().all(|c| c.is_ascii_digit())
                && (h1 - b'0') * 10 + (h2 - b'0') <= 23
                && (m1 - b'0') * 10 + (m2 - b'0') <= 59
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_numbers_normalize_to_e164() {
        assert_eq!(normalize_phone("+1 (415) 555-2671").unwrap(), "+14155552671");
        assert_eq!(normalize_phone("+44 20.7946.0958").unwrap(), "+442079460958");
        for bad in ["4155552671", "+0123456789", "+123456", "+1234567890123456", "+1415abc2671", "", "+"] {
            assert!(normalize_phone(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn dates_and_datetimes() {
        for ok in [
            "2026-03-01",
            "2024-02-29",
            "2026-03-01T09:30:00Z",
            "2026-03-01T09:30:00.123Z",
            "2026-03-01T09:30:00+05:30",
            "2026-03-01T23:59:59-08:00",
        ] {
            assert!(valid_date(ok), "{ok}");
        }
        for bad in [
            "",
            "2026-3-1",
            "2026-13-01",
            "2026-02-29",
            "2026-04-31",
            "2026-03-01T24:00:00Z",
            "2026-03-01T09:30:00",
            "2026-03-01 09:30:00Z",
            "2026-03-01T09:30:00.Z",
            "2026-03-01T09:30:00+24:00",
            "2026-03-01T09:30:00+0530",
            "2026-03-01xyz",
            "２０２６-03-01",
        ] {
            assert!(!valid_date(bad), "{bad}");
        }
    }
}
