mod problem;
mod user;

#[macro_use]
extern crate log;

pub use problem::{ProblemDb, ProblemRecord};
pub use user::{UserDb, UserRecord};

fn err_handle<T>(result: Result<T, sled::Error>) -> Option<T> {
    result
        .map_err(|e| {
            error!("db error {}", e);
            e
        })
        .ok()
}
