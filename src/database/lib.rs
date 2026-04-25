mod task;

#[macro_use]
extern crate log;

pub use task::{TaskDb, TaskRecord};

fn err_handle<T>(result: Result<T, sled::Error>) -> Option<T> {
    result.inspect_err(|e| error!("db error {e}")).ok()
}
