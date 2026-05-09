pub mod balance;
pub mod book;
pub mod impersonate;
pub mod payment;
pub mod schedule;
pub mod user;
pub mod worktime;
pub use balance::{BalanceActor, BalanceParams};
pub use book::{BookParams, BookingActor};
pub use impersonate::ImpersonateParams;
pub use payment::{PaymentActor, PaymentParams};
pub use schedule::ScheduleParams;
pub use user::{UserCommand, UserParams};
pub use worktime::WorktimeParams;

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
/// TeacherCommand, AdminCommand, and ImpersonateTeacherCommand implement this trait.
pub trait PaymentCommand: Sized + Clone {
    fn payment(params: PaymentParams) -> Self;
}

/// Trait for commands that can emit a schedule command.
pub trait ScheduleCommand: Sized + Clone {
    fn schedule(params: ScheduleParams) -> Self;
}

/// Trait for commands that can emit a balance command.
pub trait BalanceCommand: Sized + Clone {
    fn balance(params: BalanceParams) -> Self;
}

/// Trait for commands that can emit a worktime command.
/// TeacherCommand and ImpersonateTeacherCommand implement this trait.
pub trait WorktimeCommand: Sized + Clone {
    fn worktime(params: WorktimeParams) -> Self;
}
