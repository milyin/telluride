pub mod book;
pub mod payment;
pub use book::{BookParams, BookingActor};
pub use payment::PaymentParams;

/// Trait for commands that can emit a book command.
/// StudentCommand, ImpersonateCommand, and TeacherCommand implement this trait,
/// allowing the book API to be generic over the command type.
pub trait BookCommand: Sized + Clone {
    fn book(params: BookParams) -> Self;
}

/// Trait for commands that can emit a payment command.
/// Only TeacherCommand implements this trait.
pub trait PaymentCommand: Sized + Clone {
    fn payment(params: PaymentParams) -> Self;
}
