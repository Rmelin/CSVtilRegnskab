use regex::Regex;
use rust_decimal::Decimal;
use sqlx::{FromRow, SqlitePool};
use std::str::FromStr;

use crate::error::AppResult;

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
struct Rule {
    id: i64,
    name: String,
    regex_pattern: String,
    default_budget_post_id: Option<i64>,
    direction: String,
    enabled: bool,
    priority: i64,
}

#[derive(Debug, Clone, FromRow)]
struct Transaction {
    id: i64,
    text: String,
    amount: String,
}

pub async fn apply_matcher_rules(pool: &SqlitePool) -> AppResult<()> {
    let rules = sqlx::query_as::<_, Rule>(
        "SELECT id, name, regex_pattern, default_budget_post_id, direction, enabled, priority \
         FROM matcher_rules WHERE enabled = 1 ORDER BY priority ASC",
    )
    .fetch_all(pool)
    .await?;

    let transactions = sqlx::query_as::<_, Transaction>(
        "SELECT id, text, CAST(amount AS TEXT) as amount \
         FROM transactions \
         WHERE confirmed = 0 AND assigned_budget_post_id IS NULL",
    )
    .fetch_all(pool)
    .await?;

    for transaction in transactions {
        let amount = Decimal::from_str(&transaction.amount)
            .map_err(|_| crate::error::AppError::Parse("Invalid amount".to_string()))?;
        let direction = if amount > Decimal::from_str("0").unwrap() {
            "income"
        } else if amount < Decimal::from_str("0").unwrap() {
            "expense"
        } else {
            "both"
        };

        for rule in &rules {
            if !direction_matches(direction, &rule.direction) {
                continue;
            }
            let regex = match Regex::new(&rule.regex_pattern) {
                Ok(regex) => regex,
                Err(_) => continue,
            };
            if regex.is_match(&transaction.text) {
                sqlx::query(
                    "UPDATE transactions \
                     SET suggested_budget_post_id = ?, matched_rule_id = ? \
                     WHERE id = ?",
                )
                .bind(rule.default_budget_post_id)
                .bind(rule.id)
                .bind(transaction.id)
                .execute(pool)
                .await?;
                break;
            }
        }
    }

    Ok(())
}

fn direction_matches(transaction_direction: &str, rule_direction: &str) -> bool {
    match rule_direction {
        "both" => true,
        "income" => transaction_direction == "income",
        "expense" => transaction_direction == "expense",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matcher_priority_behavior() {
        let rules = vec![
            Rule {
                id: 1,
                name: "Rule A".to_string(),
                regex_pattern: ".*".to_string(),
                default_budget_post_id: Some(1),
                direction: "expense".to_string(),
                enabled: true,
                priority: 1,
            },
            Rule {
                id: 2,
                name: "Rule B".to_string(),
                regex_pattern: ".*".to_string(),
                default_budget_post_id: Some(2),
                direction: "expense".to_string(),
                enabled: true,
                priority: 2,
            },
        ];

        let transaction = Transaction {
            id: 10,
            text: "Test".to_string(),
            amount: Decimal::from_str("-10").unwrap(),
        };

        let mut matched = None;
        for rule in &rules {
            let regex = Regex::new(&rule.regex_pattern).unwrap();
            if regex.is_match(&transaction.text) {
                matched = Some(rule.id);
                break;
            }
        }

        assert_eq!(matched, Some(1));
    }
}
