mod api;
mod bot;
mod config;
mod models;
mod sheets;
mod state;
mod types;

use std::sync::Arc;
use telluride::command::CallbackKey;
use telluride::data_store::InMemStore;
use teloxide::prelude::*;

use bot::admin::AdminCommand;
use bot::impersonate::ImpersonateCommand;
use bot::student::StudentCommand;
use bot::teacher::TeacherCommand;
use config::Config;
use sheets::SheetsClient;
use state::BotState;

use crate::bot::admin::{admin_callback_command_handler, admin_command_handler};
use crate::bot::filter_callback_by;
use crate::bot::impersonate::{impersonate_callback_command_handler, impersonate_command_handler};
use crate::bot::student::{student_callback_command_handler, student_command_handler};
use crate::bot::teacher::{teacher_callback_command_handler, teacher_command_handler};

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

    // Per-command-type callback storages for inline keyboard buttons.
    let student_callbacks = Arc::new(InMemStore::<CallbackKey, StudentCommand>::new());
    let teacher_callbacks = Arc::new(InMemStore::<CallbackKey, TeacherCommand>::new());
    let impersonate_callbacks = Arc::new(InMemStore::<CallbackKey, ImpersonateCommand>::new());
    let admin_callbacks = Arc::new(InMemStore::<CallbackKey, AdminCommand>::new());

    let handler = dptree::entry()
        // Student commands (/start, /help, /schedule, /book).
        .branch(
            Update::filter_message()
                .filter_async(|msg: Message, state: Arc<BotState>| {
                    bot::filter_message_by(msg, state, bot::student::is_student)
                })
                .chain(bot::filter_command_prefixed::<StudentCommand, _>())
                .endpoint(student_command_handler),
        )
        // Teacher commands (/start, /help, /schedule, /impersonate, /admin, /refresh).
        .branch(
            Update::filter_message()
                .filter_async(|msg: Message, state: Arc<BotState>| {
                    bot::filter_message_by(msg, state, bot::teacher::is_teacher)
                })
                .chain(bot::filter_command_prefixed::<TeacherCommand, _>())
                .endpoint(teacher_command_handler),
        )
        // Impersonation mode commands (/start, /help, /schedule, /book, /quit).
        .branch(
            Update::filter_message()
                .filter_async(|msg: Message, state: Arc<BotState>| {
                    bot::filter_message_by(msg, state, bot::impersonate::is_impersonate)
                })
                .chain(bot::filter_command_prefixed::<ImpersonateCommand, _>())
                .endpoint(impersonate_command_handler),
        )
        // Admin mode commands (/start, /help, /status, /refresh, /quit).
        .branch(
            Update::filter_message()
                .filter_async(|msg: Message, state: Arc<BotState>| {
                    bot::filter_message_by(msg, state, bot::admin::is_admin)
                })
                .chain(bot::filter_command_prefixed::<AdminCommand, _>())
                .endpoint(admin_command_handler),
        )
        // Student inline keyboard callbacks.
        .branch(
            Update::filter_callback_query()
                .filter(|q: CallbackQuery| {
                    q.data
                        .as_ref()
                        .is_some_and(|d| CallbackKey::is_packed_data(d))
                })
                .filter_async(|q: CallbackQuery, state: Arc<BotState>| {
                    filter_callback_by(q, state, bot::student::is_student)
                })
                .endpoint(student_callback_command_handler),
        )
        // Teacher inline keyboard callbacks (e.g. student selection for /impersonate).
        .branch(
            Update::filter_callback_query()
                .filter(|q: CallbackQuery| {
                    q.data
                        .as_ref()
                        .is_some_and(|d| CallbackKey::is_packed_data(d))
                })
                .filter_async(|q: CallbackQuery, state: Arc<BotState>| {
                    filter_callback_by(q, state, bot::teacher::is_teacher)
                })
                .endpoint(teacher_callback_command_handler),
        )
        // Impersonation mode inline keyboard callbacks.
        .branch(
            Update::filter_callback_query()
                .filter(|q: CallbackQuery| {
                    q.data
                        .as_ref()
                        .is_some_and(|d| CallbackKey::is_packed_data(d))
                })
                .filter_async(|q: CallbackQuery, state: Arc<BotState>| {
                    filter_callback_by(q, state, bot::impersonate::is_impersonate)
                })
                .endpoint(impersonate_callback_command_handler),
        )
        // Admin mode inline keyboard callbacks.
        .branch(
            Update::filter_callback_query()
                .filter(|q: CallbackQuery| {
                    q.data
                        .as_ref()
                        .is_some_and(|d| CallbackKey::is_packed_data(d))
                })
                .filter_async(|q: CallbackQuery, state: Arc<BotState>| {
                    filter_callback_by(q, state, bot::admin::is_admin)
                })
                .endpoint(admin_callback_command_handler),
        )
        // Plain text messages (non-command) and unhandled commands (e.g. unauthorized users).
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
        .dependencies(dptree::deps![
            state,
            student_callbacks,
            teacher_callbacks,
            impersonate_callbacks,
            admin_callbacks
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
