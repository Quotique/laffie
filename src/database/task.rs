use std::{
    convert::{From, Into},
    path::Path,
};

use bincode::{Decode, Encode, config, config::Configuration};
use sled::{Db, Error};

use solver::{
    task::{Task, TermProps},
    term::TermBuf,
};

use super::err_handle;

#[derive(Clone, Encode, Decode)]
pub struct TaskRecord {
    pub id:     u128,
    pub text:   String,
    pub group:  String,
    pub givens: Vec<TermBuf>,
    pub goal:   TermBuf,

    pub answer:  Vec<TermBuf>,
    pub runs:    Vec<usize>,
    pub reports: Vec<u64>,
}

pub struct TaskDb {
    db:     Db,
    config: Configuration,
}

impl From<&Task> for TaskRecord {
    fn from(value: &Task) -> Self {
        Self {
            id:     value.id as u128,
            text:   value.text.clone(),
            group:  value.group.clone(),
            givens: Vec::from_iter(value.givens.iter().map(|x| (*x.term).clone())),
            goal:   (*value.goal.term).clone(),

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
            group:            self.group,
            givens:           Vec::from_iter(self.givens.into_iter().map(TermProps::from)),
            possible_answers: self.answer,
            goal:             TermProps::from(self.goal),
            subtask_level:    0,
        }
    }
}

impl TaskDb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Self {
            db:     sled::open(path)?,
            config: config::standard(),
        })
    }

    pub fn get(&self, id: u128) -> Result<Option<TaskRecord>, Error> {
        let key = id.to_le_bytes();
        Ok(self.db.get(key)?.map(|b| {
            let (decoded, _): (TaskRecord, usize) =
                bincode::decode_from_slice(&b[..], self.config).unwrap();
            decoded
        }))
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
            decoded
        })
    }
}
