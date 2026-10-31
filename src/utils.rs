use alloy::primitives::Address;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use validator::{ValidateEmail, ValidationErrors};

use crate::error::AppError;

/// The first field error as a single human-readable message.
pub fn first_error(errors: &ValidationErrors) -> String {
    errors
        .field_errors()
        .into_iter()
        .next()
        .and_then(|(field, errs)| {
            errs.first().map(|e| match &e.message {
                Some(msg) => msg.to_string(),
                None => format!("{} is invalid", field),
            })
        })
        .unwrap_or_else(|| "Invalid input".to_string())
}

/// `row` is 1-based, so the user can find the row they need to fix.
pub fn row_error(row: usize, msg: &str) -> AppError {
    AppError::BadRequest(format!("Row {row}: {msg}"))
}

/// Lowercase `0x...`, so the same wallet always compares equal.
pub fn clean_wallet(row: usize, wallet: &str) -> Result<String, AppError> {
    let wallet = wallet
        .trim()
        .parse::<Address>()
        .map_err(|_| row_error(row, "invalid wallet address"))?;
    if wallet == Address::ZERO {
        return Err(row_error(row, "wallet address can't be the zero address"));
    }

    Ok(format!("{wallet:#x}"))
}

/// Whole number of base units, above 0, and fits in uint256 (78 digits).
pub fn clean_amount(row: usize, amount: &str) -> Result<String, AppError> {
    let amount = amount.trim().trim_start_matches('0');
    if amount.is_empty() || amount.len() > 78 || !amount.bytes().all(|b| b.is_ascii_digit()) {
        return Err(row_error(
            row,
            "amount must be a whole number of base units above 0",
        ));
    }

    Ok(amount.to_string())
}

pub fn clean_email(row: usize, email: Option<String>) -> Result<Option<String>, AppError> {
    let email = email
        .map(|e| e.trim().to_lowercase())
        .filter(|e| !e.is_empty());
    if let Some(email) = &email
        && !email.validate_email()
    {
        return Err(row_error(row, "invalid email"));
    }

    Ok(email)
}

pub fn clean_name(row: usize, name: Option<String>) -> Result<Option<String>, AppError> {
    let name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    if name.as_ref().is_some_and(|n| n.chars().count() > 100) {
        return Err(row_error(row, "name must be at most 100 characters"));
    }

    Ok(name)
}

/// Start (inclusive) and end (exclusive); `None` means unbounded.
pub type DateRange = (Option<DateTime<Utc>>, Option<DateTime<Utc>>);

/// `from`/`to` as UTC days, turned into a half-open range: `to` includes its whole day.
pub fn date_range(from: Option<NaiveDate>, to: Option<NaiveDate>) -> Result<DateRange, AppError> {
    if let (Some(from), Some(to)) = (from, to)
        && from > to
    {
        return Err(AppError::BadRequest(
            "from must be on or before to".to_string(),
        ));
    }

    let start_of_day = |d: NaiveDate| d.and_time(NaiveTime::MIN).and_utc();

    let to = to
        .map(|d| {
            d.succ_opt()
                .map(start_of_day)
                .ok_or_else(|| AppError::BadRequest("Invalid to date".to_string()))
        })
        .transpose()?;

    Ok((from.map(start_of_day), to))
}
