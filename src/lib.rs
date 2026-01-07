mod api;

pub mod markdown {
    pub use crate::api::markdown::{
        string::{MarkdownString, MarkdownStringMessage},
        validate::validate_markdownv2_format,
    };
}
