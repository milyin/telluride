//! Payment data reader for the `Payments` sheet tab.
//!
//! Not yet implemented — returns an empty list.

use anyhow::Result;

use super::SheetsClient;
use crate::models::Payment;

impl SheetsClient {
    /// Reads payment records from the `Payments` sheet.
    ///
    /// **Not yet implemented** — always returns an empty `Vec`.
    pub async fn get_payments(&self) -> Result<Vec<Payment>> {
        // TODO: implement payment parsing (same pattern as students/teachers).
        Ok(vec![])
    }
}
