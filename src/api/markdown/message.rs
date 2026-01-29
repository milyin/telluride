use teloxide::{
    Bot,
    payloads::{
        EditMessageTextInlineSetters, EditMessageTextSetters, SendMessage, SendMessageSetters,
    },
    prelude::Requester,
    requests::JsonRequest,
    types::{
        MessageId,
        ParseMode::{self, MarkdownV2},
        Recipient,
    },
};

use super::string::MarkdownString;

/// Trait for sending markdown messages with [teloxide Bot](https://docs.rs/teloxide/latest/teloxide/struct.Bot.html)
///
/// This trait provides a convenient method for sending `MarkdownString` messages
/// using [teloxide Bot](https://docs.rs/teloxide/latest/teloxide/struct.Bot.html), automatically setting the parse mode to `MarkdownV2`.
///
/// All methods automatically validate message length and truncate with "..."
/// if the message exceeds Telegram's 4096 character limit.
///
/// # Example
///
/// ```rust
/// use telluride::markdown::{MarkdownString, MarkdownStringMessage};
/// use teloxide::{Bot, prelude::Requester, types::ChatId};
///
/// async fn send_markdown_example(bot: Bot, chat_id: ChatId) {
///     // Create a MarkdownString (safely escaped)
///     let message = MarkdownString::escape("Hello *world*!");
///
///     // Use the trait method - automatically sets ParseMode::MarkdownV2
///     let request = bot.send_markdown_message(chat_id, message);
///     request.await.unwrap();
/// }
/// ```
///
/// The trait allows you to use [`send_markdown_message`](Self::send_markdown_message) with `MarkdownString` parameters
/// while automatically applying the correct parse mode, making it safer and more
/// convenient than manually setting the parse mode each time.
#[allow(async_fn_in_trait)]
pub trait MarkdownStringMessage: Requester {
    /// This method replaces [teloxide Bot::send_message](https://docs.rs/teloxide/latest/teloxide/struct.Bot.html#method.send_message) for `MarkdownString`
    fn send_markdown_message<C>(
        &self,
        chat_id: C,
        text: MarkdownString,
    ) -> JsonRequest<SendMessage>
    where
        C: Into<Recipient>;

    /// This method replaces [teloxide Bot::edit_message_text](https://docs.rs/teloxide/latest/teloxide/struct.Bot.html#method.edit_message_text) for `MarkdownString`
    fn edit_markdown_message_text<C>(
        &self,
        chat_id: C,
        message_id: MessageId,
        text: MarkdownString,
    ) -> <Self as Requester>::EditMessageText
    where
        C: Into<Recipient>;

    /// This method replaces [teloxide Bot::edit_message_text_inline](https://docs.rs/teloxide/latest/teloxide/struct.Bot.html#method.edit_message_text_inline) for `MarkdownString`
    fn edit_markdown_message_text_inline(
        &self,
        inline_message_id: &str,
        text: MarkdownString,
    ) -> <Self as Requester>::EditMessageTextInline;
}

/// Implementation of `MarkdownStringMessage` for teloxide `Bot`
impl MarkdownStringMessage for Bot {
    fn send_markdown_message<C>(&self, chat_id: C, text: MarkdownString) -> JsonRequest<SendMessage>
    where
        C: Into<Recipient>,
    {
        self.send_message(chat_id, text)
            .parse_mode(ParseMode::MarkdownV2)
    }

    fn edit_markdown_message_text<C>(
        &self,
        chat_id: C,
        message_id: MessageId,
        text: MarkdownString,
    ) -> <Self as Requester>::EditMessageText
    where
        C: Into<Recipient>,
    {
        self.edit_message_text(chat_id, message_id, text)
            .parse_mode(MarkdownV2)
    }

    fn edit_markdown_message_text_inline(
        &self,
        inline_message_id: &str,
        text: MarkdownString,
    ) -> <Self as Requester>::EditMessageTextInline {
        self.edit_message_text_inline(inline_message_id, text)
            .parse_mode(MarkdownV2)
    }
}

#[cfg(test)]
mod tests {
    use super::{MarkdownString, MarkdownStringMessage};

    // Note: We can't easily test the MarkdownStringSendMessage trait without
    // setting up a real Bot instance, but we can test that the types are correct
    #[test]
    fn test_markdown_string_send_message_trait_exists() {
        // If this compiles, the trait is properly defined
        fn _test_trait_bound<T: MarkdownStringMessage>(_bot: T) {}

        // Test that MarkdownString can be created for the trait method
        let _message = MarkdownString::escape("Test message");
    }
}
