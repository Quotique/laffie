use std::{
    convert::{From, Into},
    path::Path,
};

use bincode::{config, config::Configuration, Decode, Encode};
use sled::{Db, Error};

use solver::{
    task::{Task, TermProps},
    term::Term,
};

use super::err_handle;

const VERSION: u64 = 2;

pub mod old {
    use super::*;

    pub const VERSION: u64 = 1;

    #[derive(Clone, Encode, Decode)]
    pub struct TaskRecord {
        pub version: u64,

        pub id:     u128,
        pub givens: Vec<Term>,
        pub goal:   Term,

        pub answer:  Term,
        pub runs:    Vec<usize>,
        pub reports: Vec<u64>,
    }
}

#[derive(Clone, Encode, Decode)]
pub struct TaskRecord {
    pub version: u64,

    pub id:     u128,
    pub text:   String,
    pub group:  String,
    pub givens: Vec<Term>,
    pub goal:   Term,

    pub answer:  Vec<Term>,
    pub runs:    Vec<usize>,
    pub reports: Vec<u64>,
}

pub struct TaskDb {
    db:     Db,
    config: Configuration,
}

impl From<old::TaskRecord> for TaskRecord {
    fn from(value: old::TaskRecord) -> Self {
        Self {
            version: VERSION,
            id:      value.id,
            text:    Default::default(),
            group:   Default::default(),
            givens:  value.givens,
            goal:    value.goal,

            answer:  vec![value.answer],
            runs:    value.runs,
            reports: Default::default(),
        }
    }
}

impl From<&Task> for TaskRecord {
    fn from(value: &Task) -> Self {
        Self {
            version: VERSION,
            id:      value.id as u128,
            text:    value.text.clone(),
            group:   value.group.clone(),
            givens:  Vec::from_iter(value.givens.iter().map(|x| (*x.term).clone())),
            goal:    (*value.goal.term).clone(),

            answer:  value.possible_answers.clone(),
            runs:    Default::default(),
            reports: Default::default(),
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<Task> for TaskRecord {
    fn into(self) -> Task {
        Task {
            id:               self.id as u64,
            text:             self.text,
            group:            "".to_string(),
            givens:           Vec::from_iter(self.givens.into_iter().map(TermProps::from)),
            possible_answers: self.answer,
            goal:             TermProps::from(self.goal),
            subtask_level:    0,
        }
    }
}

impl TaskRecord {
    fn is_same(&self, other: &Self) -> bool {
        if self.goal != other.goal {
            return false;
        }
        for i in self.givens.iter() {
            if !other.givens.iter().any(|x| x == i) {
                return false;
            }
        }
        for i in other.givens.iter() {
            if !self.givens.iter().any(|x| x == i) {
                return false;
            }
        }
        true
    }
}

impl TaskDb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Self {
            db:     sled::open(path)?,
            config: config::standard(),
        })
    }

    pub fn backup<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let backup = sled::open(path)?;

        for (k, v) in self.db.iter().flat_map(err_handle) {
            backup.insert(k, v)?;
        }

        Ok(())
    }

    pub fn restore<P: AsRef<Path>>(backup_path: P, db_path: P) -> Result<(), Error> {
        let db = sled::open(db_path)?;
        db.clear()?;

        let backup = sled::open(backup_path)?;

        for (k, v) in backup.iter().flat_map(err_handle) {
            db.insert(k, v)?;
        }
        Ok(())
    }

    pub fn get(&self, id: u128) -> Result<Option<TaskRecord>, Error> {
        let key = id.to_le_bytes();
        Ok(self.db.get(key)?.map(|b| {
            let (decoded, _): (TaskRecord, usize) =
                bincode::decode_from_slice(&b[..], self.config).unwrap();
            decoded
        }))
    }

    pub fn get_or_insert(&self, mut task: TaskRecord) -> Result<TaskRecord, Error> {
        let (_, task_id) = split_id(task.id);

        for number in 1..u64::MAX {
            let id = compose_id(number, task_id);
            match self.get(id)? {
                Some(p) => {
                    if p.is_same(&task) {
                        return Ok(p);
                    }
                }
                None => {
                    task.id = id;
                    self.put(&task)?;
                    return Ok(task);
                }
            }
        }
        unreachable!()
    }

    pub fn put(&self, task: &TaskRecord) -> Result<(), Error> {
        let key = task.id.to_le_bytes();
        let encoded: Vec<u8> = bincode::encode_to_vec(task, self.config).unwrap();

        self.db.insert(key, encoded).map(|_| ())
    }

    pub fn remove(&self, id: i128) -> Result<(), Error> {
        let key = id.to_le_bytes();
        self.db.remove(key).map(|_| ())
    }

    pub fn iter(&self) -> impl Iterator<Item = TaskRecord> + '_ {
        self.db.iter().flat_map(err_handle).map(|(_, v)| {
            let (decoded, _): (TaskRecord, usize) =
                bincode::decode_from_slice(&v[..], self.config).unwrap();
            assert_eq!(decoded.version, VERSION);
            decoded
        })
    }

    pub fn iter_old(&self) -> impl Iterator<Item = old::TaskRecord> + '_ {
        self.db.iter().flat_map(err_handle).map(|(_, v)| {
            let (decoded, _): (old::TaskRecord, usize) =
                bincode::decode_from_slice(&v[..], self.config).unwrap();
            assert_eq!(decoded.version, old::VERSION);
            decoded
        })
    }
}

fn compose_id(number: u64, task_id: u64) -> u128 {
    ((number as u128) << 64) + task_id as u128
}

fn split_id(id: u128) -> (u64, u64) {
    (
        ((id & 0xff_ff_ff_ff_ff_ff_ff_ff_00_00_00_00_00_00_00_00) >> 64) as u64,
        (id & 0x00_00_00_00_00_00_00_00_ff_ff_ff_ff_ff_ff_ff_ff) as u64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_test() {
        for (number, task_id) in &[(0, 0), (1, 1000), (100, 5000)] {
            let id = compose_id(*number, *task_id);
            let (decoded_number, decoded_task_id) = split_id(id);
            assert_eq!(decoded_number, *number);
            assert_eq!(decoded_task_id, *task_id);
        }
    }
}
