mod api;

pub mod calendar {
    pub use crate::api::calendar::build_month_calendar;
}

/// The `markdown` module provides utilities for safe working with MarkdownV2 formatted strings.
/// The goal is to make it impossible to create invalid MarkdownV2 strings that will cause runtime errors.
///
/// It provides the type [`MarkdownString`](markdown::MarkdownString) which is
/// compile-time validated to ensure correct MarkdownV2 formatting.
/// This goal is achieved by disallowing direct construction of `MarkdownString` from arbitrary strings.
/// Instead the following methods are provided:
///
/// - [`markdown_string!`] macro: Allows creation of `MarkdownString` from string literals
///   with compile-time validation.
///
/// - [`markdown_format!`] macro: Similar to [`format!`], but validates the pattern at compile-time,
///   automatically escapes the arguments, and supports special prefixes `@raw` and `@code` to avoid escaping.
///
///
/// The trait [`MarkdownStringMessage`](markdown::MarkdownStringMessage) provides methods
/// [`send_markdown_message`](markdown::MarkdownStringMessage::send_markdown_message) and
/// [`edit_markdown_message_text`](markdown::MarkdownStringMessage::edit_markdown_message_text)
/// which are similar to [teloxide](https://docs.rs/teloxide/latest/teloxide/)'s
///[Bot::send_message](https://docs.rs/teloxide/latest/teloxide/struct.Bot.html#method.send_message) and
/// [Bot::edit_message_text](https://docs.rs/teloxide/latest/teloxide/struct.Bot.html#method.edit_message_text) respectively,
/// but accept [`MarkdownString`](markdown::MarkdownString``) and automatically set the parse mode to `MarkdownV2`.
/// The teloxide [Bot](https://docs.rs/teloxide/latest/teloxide/struct.Bot.html) type is extended with this trait implementation.
pub mod markdown {
    pub use crate::api::markdown::{
        message::MarkdownStringMessage, string::MarkdownString,
        validate::validate_markdownv2_format,
    };
}

pub mod command {
    pub use crate::api::command::{
        button_extensions::InlineKeyboardButtonPackedExt,
        callback_errors::UnpackError,
        callback_packing::{CallbackBitcode, CallbackEncode, CallbackKey, MAX_CALLBACK_DATA_SIZE},
    };
}

pub mod data_store {
    pub use crate::api::data_store::{
        data_store_trait::{CommonProxy, DataStoreTrait, UserDataStoreTrait, UserProxy},
        file_system_yaml::FilesystemYamlStore,
        in_mem::InMemStore,
        util::{decode_filename_to_key, encode_key_to_filename},
    };
}

pub mod utils {
    pub use crate::api::utils::{ParamParser, screen_spaces, split_with_screened_spaces};
    pub use crate::format_screen_spaces;
}
