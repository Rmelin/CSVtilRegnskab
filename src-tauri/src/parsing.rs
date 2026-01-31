use chrono::NaiveDate;
use regex::Regex;
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::error::{AppError, AppResult};

const KONTINGENT_RULE_NAME: &str = "Kontingent - SE MEDD.";

pub fn parse_danish_decimal(value: &str) -> AppResult<Decimal> {
    let cleaned = value
        .trim()
        .replace('.', "")
        .replace(',', ".");
    Decimal::from_str(&cleaned)
        .map_err(|_| AppError::Parse(format!("Invalid decimal: {}", value)))
}

pub fn parse_danish_date(value: &str) -> AppResult<NaiveDate> {
    let trimmed = value.trim();
    NaiveDate::parse_from_str(trimmed, "%d.%m.%Y")
        .or_else(|_| NaiveDate::parse_from_str(trimmed, "%d-%m-%Y"))
        .map_err(|_| AppError::Parse(format!("Invalid date: {}", value)))
}

#[derive(Debug, Clone)]
pub struct KontingentInfo {
    pub member_id: String,
    pub member_name: String,
}

pub fn kontingent_regex() -> Regex {
    Regex::new(r"^\s*(\d{6,10})\s+(.+?)\s+-SE\s+MEDD\.\s*$").expect("valid regex")
}

pub fn parse_kontingent_info(text: &str, matched_rule_name: Option<&str>) -> Option<KontingentInfo> {
    if matched_rule_name != Some(KONTINGENT_RULE_NAME) {
        return None;
    }
    let regex = kontingent_regex();
    let captures = regex.captures(text)?;
    Some(KontingentInfo {
        member_id: captures.get(1)?.as_str().to_string(),
        member_name: captures.get(2)?.as_str().trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_danish_decimal_samples() {
        let value = parse_danish_decimal("30.675,12").unwrap();
        assert_eq!(value, Decimal::from_str("30675.12").unwrap());
        let value = parse_danish_decimal("-11.040,00").unwrap();
        assert_eq!(value, Decimal::from_str("-11040.00").unwrap());
    }

    #[test]
    fn parse_dates() {
        let date = parse_danish_date("30.12.2025").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 12, 30).unwrap());
        let date = parse_danish_date("02-01-2025").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 1, 2).unwrap());
    }

    #[test]
    fn kontingent_regex_matches_samples() {
        let regex = kontingent_regex();
        let samples = [
            "2789610 Pia Hansen B                -SE MEDD.",
            "3578349 Sebastian Ej                -SE MEDD.",
            "4056117 Vinnie David                -SE MEDD.",
        ];
        for sample in &samples {
            assert!(regex.is_match(sample));
        }
    }
}
