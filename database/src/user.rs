use std::{collections::BTreeSet, iter::Iterator, path::Path};

use bincode::{config, config::Configuration, Decode, Encode};
use rocksdb::{
    backup::{BackupEngine, BackupEngineOptions},
    Error, IteratorMode, DB,
};

const BACKUPS_COUNT: usize = 3;
const VERSION: u64 = 2;

pub mod old {
    use super::*;

    pub const VERSION: u64 = 2;

    #[derive(Encode, Decode)]
    pub struct UserRecord {
        pub version: u64,

        pub id:       u64,
        pub locale:   String,
        pub problems: BTreeSet<u128>,
    }
}

#[derive(Encode, Decode)]
pub struct UserRecord {
    pub version: u64,

    pub id:     u64,
    pub locale: String,
    problems:   BTreeSet<u128>,
}

pub struct UserDb {
    db:     DB,
    config: Configuration,
}

impl From<old::UserRecord> for UserRecord {
    fn from(value: old::UserRecord) -> Self {
        Self {
            version:  VERSION,
            id:       value.id,
            locale:   "ru".to_owned(),
            problems: value.problems,
        }
    }
}

impl UserRecord {
    pub fn new(id: u64) -> Self {
        Self {
            version: VERSION,
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

    pub fn backup<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let backup_opts = BackupEngineOptions::default();
        let mut backup_engine = BackupEngine::open(&backup_opts, &path)?;
        backup_engine.create_new_backup_flush(&self.db, true)?;

        backup_engine.verify_backup(1)?;
        backup_engine.purge_old_backups(BACKUPS_COUNT)
    }

    pub fn restore<P: AsRef<Path>>(backup_path: P, db_path: P) -> Result<(), Error> {
        let backup_opts = BackupEngineOptions::default();
        let mut backup_engine = BackupEngine::open(&backup_opts, &backup_path).unwrap();
        let restore_option = rocksdb::backup::RestoreOptions::default();

        backup_engine.restore_from_latest_backup(&db_path, &db_path, &restore_option)
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

    pub fn iter_old(&self) -> impl Iterator<Item = old::UserRecord> + '_ {
        self.db.iterator(IteratorMode::Start).map(|(_, v)| {
            let (decoded, _): (old::UserRecord, usize) =
                bincode::decode_from_slice(&v[..], self.config).unwrap();
            assert_eq!(decoded.version, old::VERSION);
            decoded
        })
    }
}
