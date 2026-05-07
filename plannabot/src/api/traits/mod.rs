pub mod book;
pub use book::{BookParams, BookSubcmd, BookingActor};

/// Trait for commands that can emit a book command.
/// StudentCommand, ImpersonateCommand, and TeacherCommand implement this trait,
/// allowing the book API to be generic over the command type.
pub trait BookCommand: Sized + Clone {
    fn book(params: BookParams) -> Self;
}
