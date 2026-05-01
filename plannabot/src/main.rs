mod bot;
mod config;
mod models;
mod sheets;
mod state;

use std::sync::Arc;
use teloxide::prelude::*;

use config::Config;
use sheets::SheetsClient;
use state::BotState;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting Plannabot...");

    // Load configuration from environment / .env file.
    let config = Config::from_env().expect("Failed to load configuration");

    // Build the Google Sheets client (authenticates with the service account).
    let sheets = Arc::new(
        SheetsClient::new(&config.google_credentials_path, config.spreadsheet_id)
            .await
            .expect("Failed to initialise SheetsClient"),
    );

    // Ensure all required sheet tabs and columns exist (creates them if missing).
    sheets
        .ensure_all_sheets()
        .await
        .expect("Failed to ensure spreadsheet schema");

    // Initialise shared bot state and perform the first data load.
    let state = Arc::new(BotState::new(sheets));
    state
        .refresh()
        .await
        .expect("Failed to load initial data from spreadsheet");

    let bot = Bot::from_env();

    let handler = dptree::entry()
        // Common commands (/start, /help, /schedule) — available to all users.
        .branch(
            Update::filter_message()
                .filter_command::<bot::CommonCommand>()
                .endpoint(bot::common_command_handler),
        )
        // Teacher-only commands (/impersonate, /quit).
        .branch(
            Update::filter_message()
                .filter_command::<bot::TeacherCommand>()
                .endpoint(bot::teacher_command_handler),
        )
        // Plain text messages (non-command)
        .branch(
            Update::filter_message()
                .filter(|msg: Message| msg.text().is_some())
                .endpoint(bot::message_handler),
        )
        // Fallback: log and ignore everything else
        .endpoint(|update: Update| async move {
            log::debug!("Unhandled update: {:?}", update.kind);
            respond(())
        });

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
