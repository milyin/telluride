/// Trait for commands that can emit a book command.
/// Both StudentCommand and ImpersonateCommand implement this trait,
/// allowing the book API to be generic over the command type.
pub trait BookCommand: Sized + Clone {
    fn book(teacher_name: String) -> Self;
}
