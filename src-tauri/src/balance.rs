use chrono::NaiveDate;
use rust_decimal::Decimal;

pub fn build_balance_curve(
    start: NaiveDate,
    end: NaiveDate,
    initial_balance: Decimal,
    rows: Vec<(NaiveDate, NaiveDate, Decimal, i64)>,
) -> Vec<(NaiveDate, Decimal)> {
    let mut last_balance = initial_balance;
    let mut daily_last: std::collections::BTreeMap<NaiveDate, (Decimal, NaiveDate, i64)> =
        std::collections::BTreeMap::new();

    for (date, value_date, balance, id) in rows {
        match daily_last.get(&date) {
            Some((_, existing_value_date, existing_id)) => {
                if value_date > *existing_value_date
                    || (value_date == *existing_value_date && id > *existing_id)
                {
                    daily_last.insert(date, (balance, value_date, id));
                }
            }
            None => {
                daily_last.insert(date, (balance, value_date, id));
            }
        }
    }

    let mut result = Vec::new();
    let mut current = start;
    while current <= end {
        if let Some((balance, _, _)) = daily_last.get(&current) {
            last_balance = *balance;
        }
        result.push((current, last_balance));
        current = current.succ_opt().unwrap();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn curve_fills_missing_days() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let rows = vec![
            (
                start,
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                Decimal::from_str("100.00").unwrap(),
                1,
            ),
            (
                NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
                NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
                Decimal::from_str("120.00").unwrap(),
                2,
            ),
        ];
        let result = build_balance_curve(start, end, Decimal::from_str("0").unwrap(), rows);
        assert_eq!(result.len(), 3);
        assert_eq!(result[1].1, Decimal::from_str("100.00").unwrap());
        assert_eq!(result[2].1, Decimal::from_str("120.00").unwrap());
    }

    #[test]
    fn last_transaction_wins_same_day() {
        let day = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let rows = vec![
            (
                day,
                NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                Decimal::from_str("100.00").unwrap(),
                1,
            ),
            (
                day,
                NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                Decimal::from_str("150.00").unwrap(),
                2,
            ),
        ];
        let result = build_balance_curve(day, day, Decimal::from_str("0").unwrap(), rows);
        assert_eq!(result[0].1, Decimal::from_str("150.00").unwrap());
    }

    #[test]
    fn curve_has_full_year_length() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        let result = build_balance_curve(start, end, Decimal::from_str("0").unwrap(), vec![]);
        assert_eq!(result.len(), 365);
    }

    #[test]
    fn initial_balance_from_first_day() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let rows = vec![
            (
                NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                Decimal::from_str("35320.87").unwrap(),
                1,
            ),
        ];
        let initial = Decimal::from_str("35507.12").unwrap();
        let result = build_balance_curve(start, end, initial, rows);
        assert_eq!(result[0].1, Decimal::from_str("35507.12").unwrap());
    }
}
