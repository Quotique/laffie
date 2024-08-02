use std::{collections::BTreeSet, iter::Iterator, path::Path};

use bincode::{config, config::Configuration, Decode, Encode};
use sled::{Db, Error};

use super::err_handle;

const VERSION: u64 = 2;

pub mod old {
    use super::*;

    pub const VERSION: u64 = 2;

    #[derive(Encode, Decode)]
    pub struct UserRecord {
        pub version: u64,

        pub id:     u64,
        pub locale: String,
        pub tasks:  BTreeSet<u128>,
    }
}

#[derive(Encode, Decode)]
pub struct UserRecord {
    pub version: u64,

    pub id:     u64,
    pub locale: String,
    tasks:      BTreeSet<u128>,
}

pub struct UserDb {
    db:     Db,
    config: Configuration,
}

impl From<old::UserRecord> for UserRecord {
    fn from(value: old::UserRecord) -> Self {
        Self {
            version: VERSION,
            id:      value.id,
            locale:  "ru".to_owned(),
            tasks:   value.tasks,
        }
    }
}

impl UserRecord {
    pub fn new(id: u64) -> Self {
        Self {
            version: VERSION,
            id,
            locale: "ru".to_owned(),
            tasks: Default::default(),
        }
    }

    pub fn add_task_id(&mut self, id: u128) {
        self.tasks.insert(id);
    }

    pub fn tasks_iter(&self) -> impl Iterator<Item = u128> + '_ {
        self.tasks.iter().copied()
    }
}

impl UserDb {
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

        self.db.insert(key, encoded).map(|_| ())
    }

    pub fn iter_old(&self) -> impl Iterator<Item = old::UserRecord> + '_ {
        self.db.iter().flat_map(err_handle).map(|(_, v)| {
            let (decoded, _): (old::UserRecord, usize) =
                bincode::decode_from_slice(&v[..], self.config).unwrap();
            assert_eq!(decoded.version, old::VERSION);
            decoded
        })
    }
}
