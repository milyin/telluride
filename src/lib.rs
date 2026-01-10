mod api;

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
        string::{MarkdownString, MarkdownStringMessage},
        validate::validate_markdownv2_format,
    };
}

pub mod command {
    pub use crate::api::command::command_trait::{
        CommandTrait, NoopCommand,
    };
    pub use crate::api::command::command_arg::
    {
        EmptyArg, ParseCommandArg
    };
    pub use crate::api::command::command_button::{
        CallbackData, CallbackDataStorage, CallbackDataStorageTrait,
        unpack_callback_data, pack_callback_data, ButtonData,
    };
    pub use crate::api::command::command_reply_target::{
        CommandReplyTarget,
    };
}

pub mod data_store {
    pub use crate::api::data_store::{
        data_store_trait::DataStoreTrait,
        in_mem::InMemStore,
        file_system_yaml::FilesystemYamlStore,
    };
}