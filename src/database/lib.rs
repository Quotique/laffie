mod task;
mod user;

#[macro_use]
extern crate log;

pub use task::{TaskDb, TaskRecord};
pub use user::{UserDb, UserRecord};

fn err_handle<T>(result: Result<T, sled::Error>) -> Option<T> {
    result.inspect_err(|e| error!("db error {e}")).ok()
}
