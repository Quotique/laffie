use std::{collections::BTreeSet, iter::Iterator, path::Path};

use bincode::{config, config::Configuration, Decode, Encode};
use rocksdb::{Error, DB};

#[derive(Encode, Decode)]
pub struct UserRecord {
    pub id:     u64,
    pub locale: String,
    problems:   BTreeSet<u128>,
}

pub struct UserDb {
    db:     DB,
    config: Configuration,
}

impl UserRecord {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            locale: "ru".to_owned(),
            problems: Default::default(),
        }
    }

    pub fn add_problem_id(&mut self, id: u128) {
        self.problems.insert(id);
    }

    pub fn problems_iter(&self) -> impl Iterator<Item = u128> + '_ {
        self.problems.iter().copied()
    }
}

impl UserDb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Self {
            db:     DB::open_default(path)?,
            config: config::standard(),
        })
    }

    pub fn get(&self, id: u64) -> Result<Option<UserRecord>, Error> {
        let key = id.to_le_bytes();
        Ok(self.db.get(key)?.map(|b| {
            let (decoded, _): (UserRecord, usize) =
                bincode::decode_from_slice(&b[..], self.config).unwrap();
            decoded
        }))
    }

    pub fn put(&self, user: &UserRecord) -> Result<(), Error> {
        let key = user.id.to_le_bytes();

        let encoded: Vec<u8> = bincode::encode_to_vec(user, self.config).unwrap();

        self.db.put(key, encoded)
    }
}
