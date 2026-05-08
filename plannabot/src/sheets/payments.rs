//! Payment data reader for the `Payments` sheet tab.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use super::{SheetSchema, SheetsClient, PAYMENTS_COLS, SHEET_PAYMENTS};
use crate::models::{Payment, SheetParseError, TelegramName};

impl SheetsClient {
    /// Reads all rows from the `Payments` sheet and returns them as a
    /// `Vec<Payment>`, together with any [`SheetParseError`]s encountered
    /// while parsing rows.
    pub async fn get_payments(&self) -> Result<(Vec<Payment>, Vec<SheetParseError>)> {
        let range = format!("{SHEET_PAYMENTS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Payments sheet data")?;

        if rows.is_empty() {
            return Ok((vec![], vec![]));
        }

        let schema = SheetSchema::new(SHEET_PAYMENTS.to_string(), rows[0].clone());

        let mut payments: Vec<Payment> = Vec::new();
        let mut errors: Vec<SheetParseError> = Vec::new();

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }

            let row_num = row_idx + 1;

            let Some(student_telegram) = schema.get_field::<Option<TelegramName>>(
                row,
                row_num,
                Payment::STUDENT_TELEGRAM,
                &mut errors,
            ) else {
                continue;
            };

            let Some(date) = schema.get_field::<Option<DateTime<Utc>>>(
                row,
                row_num,
                Payment::DATE,
                &mut errors,
            ) else {
                continue;
            };
            let sum: i64 = schema.get_field(row, row_num, Payment::SUM, &mut errors);

            payments.push(Payment {
                student_telegram,
                date,
                sum,
                custom: schema.get_custom(row, PAYMENTS_COLS),
            });
        }

        Ok((payments, errors))
    }

    /// Returns all payment records for the given student.
    pub async fn get_payments_for_student(
        &self,
        telegram_name: &TelegramName,
    ) -> Result<Vec<Payment>> {
        let (all, _) = self.get_payments().await?;
        Ok(all
            .into_iter()
            .filter(|p| &p.student_telegram == telegram_name)
            .collect())
    }

    /// Deletes a payment row from the Payments sheet.
    /// Matches by student_telegram and exact date (DateTime<Utc>).
    pub async fn delete_payment(
        &self,
        student_telegram: &TelegramName,
        date: DateTime<Utc>,
    ) -> Result<()> {
        let range = format!("{SHEET_PAYMENTS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Payments sheet data")?;

        if rows.is_empty() {
            return Err(anyhow::anyhow!("Payments sheet is empty"));
        }

        let schema = SheetSchema::new(SHEET_PAYMENTS.to_string(), rows[0].clone());
        let mut errors: Vec<SheetParseError> = Vec::new();

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }

            let row_num = row_idx + 1;
            let Some(row_student) = schema.get_field::<Option<TelegramName>>(
                row,
                row_num,
                Payment::STUDENT_TELEGRAM,
                &mut errors,
            ) else {
                continue;
            };
            if &row_student != student_telegram {
                continue;
            }

            let Some(row_date) = schema.get_field::<Option<DateTime<Utc>>>(
                row,
                row_num,
                Payment::DATE,
                &mut errors,
            ) else {
                continue;
            };
            if row_date != date {
                continue;
            }

            self.delete_row(SHEET_PAYMENTS, row_idx).await?;
            return Ok(());
        }

        Err(anyhow::anyhow!(
            "No payment found for {} at {}",
            student_telegram,
            date
        ))
    }

    /// Appends a new payment record to the Payments sheet.
    pub async fn add_payment(
        &self,
        student_telegram: &TelegramName,
        date: DateTime<Utc>,
        sum: i64,
    ) -> Result<()> {
        let header_range = format!("{SHEET_PAYMENTS}!1:1");
        let rows = self
            .get_values(&header_range)
            .await
            .context("Failed to read Payments header row")?;
        let headers = rows.into_iter().next().unwrap_or_default();

        let date_str = date.format("%Y-%m-%d %H:%M:%S").to_string();
        let mut values: HashMap<&str, String> = HashMap::new();
        values.insert("student_telegram", student_telegram.as_str().to_string());
        values.insert("date", date_str);
        values.insert("sum", sum.to_string());

        let row: Vec<serde_json::Value> = headers
            .iter()
            .map(|h| {
                serde_json::Value::String(
                    values.get(h.as_str()).cloned().unwrap_or_default(),
                )
            })
            .collect();

        self.append_row(SHEET_PAYMENTS, row).await
    }
}
