pub mod book;
pub mod impersonate;
pub mod payment;
pub mod schedule;
pub use book::{BookParams, BookingActor};
pub use impersonate::ImpersonateParams;
pub use payment::PaymentParams;
pub use schedule::ScheduleParams;

/// Trait for commands that can emit a book command.
/// StudentCommand, ImpersonateCommand, and TeacherCommand implement this trait,
/// allowing the book API to be generic over the command type.
pub trait BookCommand: Sized + Clone {
    fn book(params: BookParams) -> Self;
}

/// Trait for commands that can emit an impersonate command.
/// Only AdminCommand implements this trait.
pub trait ImpersonateCommand: Sized + Clone {
    fn impersonate(params: ImpersonateParams) -> Self;
}

/// Trait for commands that can emit a payment command.
/// Only TeacherCommand implements this trait.
pub trait PaymentCommand: Sized + Clone {
    fn payment(params: PaymentParams) -> Self;
}

/// Trait for commands that can emit a schedule command.
pub trait ScheduleCommand: Sized + Clone {
    fn schedule(params: ScheduleParams) -> Self;
}
