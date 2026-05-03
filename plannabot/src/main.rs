mod api;
mod bot;
mod config;
mod models;
mod sheets;
mod state;

use std::sync::Arc;
use telluride::command::CallbackKey;
use telluride::data_store::InMemStore;
use teloxide::prelude::*;

use bot::Action;
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

    // Per-user callback action storage for inline keyboard buttons.
    let callback_storage = Arc::new(InMemStore::<CallbackKey, Action>::new());

    let handler = dptree::entry()
        // Common commands (/start, /help, /schedule) — available to all users.
        .branch(
            Update::filter_message()
                .filter_command::<bot::CommonCommand>()
                .endpoint(bot::common_command_handler),
        )
        // Teacher-only commands (/impersonate, /quit, /admin, /refresh) — not in impersonation mode.
        .branch(
            Update::filter_message()
                .filter_async(bot::is_teacher_not_in_impersonation_mode)
                .filter_command::<bot::TeacherCommand>()
                .endpoint(bot::teacher_command_handler),
        )
        // Admin-only commands (/status, /refresh).
        .branch(
            Update::filter_message()
                .filter_async(bot::is_teacher_in_admin_mode)
                .filter_command::<bot::AdminCommand>()
                .endpoint(bot::admin_command_handler),
        )
        // Impersonation mode commands (/quit).
        .branch(
            Update::filter_message()
                .filter_async(bot::is_teacher_in_impersonation_mode)
                .filter_command::<bot::ImpersonateCommand>()
                .endpoint(bot::impersonate_command_handler),
        )
        // Inline keyboard button presses (student selection for /impersonate).
        .branch(
            Update::filter_callback_query()
                .filter(|q: CallbackQuery| {
                    q.data
                        .as_ref()
                        .map_or(false, |data| CallbackKey::is_packed_data(data))
                })
                .endpoint(bot::callback_action_handler),
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
        .dependencies(dptree::deps![state, callback_storage])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
