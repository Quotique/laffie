use std::path::Path;

use eyre::{Result, WrapErr, bail};
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};

use crate::{codec, id::TaskId, run::Run, task::Task};

/// Maximum number of runs retained per task. When [`Db::add_run`] would
/// exceed this, the oldest run for the task is evicted.
pub const RUNS_PER_TASK_LIMIT: usize = 10;

/// On-disk schema version; bump on any incompatible layout/id/codec change.
/// [`Db::open`] refuses a file with a different version.
pub const SCHEMA_VERSION: u64 = 3;

const TASKS: TableDefinition<&[u8; 16], &[u8]> = TableDefinition::new("tasks");
const RUNS: TableDefinition<&[u8; 24], &[u8]> = TableDefinition::new("runs");
const META: TableDefinition<&str, u64> = TableDefinition::new("meta");
const SCHEMA_KEY: &str = "schema_version";

pub struct Db {
    inner: Database,
}

impl Db {
    /// Opens (creating if missing) the database file at `path`. Refuses a file
    /// with a mismatched, or missing-but-non-empty, [`SCHEMA_VERSION`].
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let inner =
            Database::create(path).wrap_err_with(|| format!("redb open at {}", path.display()))?;

        let tx = inner.begin_write().wrap_err("init: begin_write")?;
        {
            let _ = tx.open_table(RUNS).wrap_err("init: open runs table")?;
            let task_count = tx
                .open_table(TASKS)
                .wrap_err("init: open tasks table")?
                .len();
            let task_count = task_count.wrap_err("init: tasks len")?;

            let mut meta = tx.open_table(META).wrap_err("init: open meta table")?;
            let stored = meta
                .get(SCHEMA_KEY)
                .wrap_err("init: read version")?
                .map(|v| v.value());
            match stored {
                Some(v) if v == SCHEMA_VERSION => {}
                Some(v) => bail!(
                    "db at {} has schema version {v}, expected {SCHEMA_VERSION}; \
                     back it up and recreate",
                    path.display()
                ),
                None if task_count > 0 => bail!(
                    "db at {} predates schema versioning (no version marker); \
                     back it up and recreate",
                    path.display()
                ),
                None => {
                    meta.insert(SCHEMA_KEY, SCHEMA_VERSION)
                        .wrap_err("init: write version")?;
                }
            }
        }
        tx.commit().wrap_err("init: commit")?;

        Ok(Self { inner })
    }

    pub fn put_task(&self, task: &Task) -> Result<()> {
        let bytes = codec::encode(task)?;
        let tx = self.inner.begin_write().wrap_err("put_task: begin_write")?;
        {
            let mut t = tx.open_table(TASKS).wrap_err("put_task: open table")?;
            t.insert(&task.id, bytes.as_slice())
                .wrap_err("put_task: insert")?;
        }
        tx.commit().wrap_err("put_task: commit")?;
        Ok(())
    }

    pub fn get_task(&self, id: TaskId) -> Result<Option<Task>> {
        let tx = self.inner.begin_read().wrap_err("get_task: begin_read")?;
        let t = tx.open_table(TASKS).wrap_err("get_task: open table")?;
        match t.get(&id).wrap_err("get_task: read")? {
            Some(v) => Ok(Some(codec::decode(v.value())?)),
            None => Ok(None),
        }
    }

    /// Eagerly collects every persisted task. The expected scale (~10^5)
    /// makes streaming unnecessary; switch to a cursor if that ever
    /// changes.
    pub fn iter_tasks(&self) -> Result<Vec<Task>> {
        let tx = self.inner.begin_read().wrap_err("iter_tasks: begin_read")?;
        let t = tx.open_table(TASKS).wrap_err("iter_tasks: open table")?;
        let mut out = Vec::new();
        for entry in t.iter().wrap_err("iter_tasks: iter")? {
            let (_, v) = entry.wrap_err("iter_tasks: row")?;
            out.push(codec::decode(v.value())?);
        }
        Ok(out)
    }

    pub fn remove_task(&self, id: TaskId) -> Result<()> {
        let tx = self
            .inner
            .begin_write()
            .wrap_err("remove_task: begin_write")?;
        {
            let mut t = tx.open_table(TASKS).wrap_err("remove_task: open table")?;
            t.remove(&id).wrap_err("remove_task: tasks remove")?;
        }
        {
            let mut r = tx.open_table(RUNS).wrap_err("remove_task: open runs")?;
            let lo = run_key(id, 0);
            let hi = run_key(id, u64::MAX);
            let stale: Vec<[u8; 24]> = r
                .range::<&[u8; 24]>(&lo..=&hi)
                .wrap_err("remove_task: range")?
                .filter_map(|e| e.ok().map(|(k, _)| *k.value()))
                .collect();
            for k in stale {
                r.remove(&k).wrap_err("remove_task: runs remove")?;
            }
        }
        tx.commit().wrap_err("remove_task: commit")?;
        Ok(())
    }

    pub fn set_hidden(&self, id: TaskId, hidden: bool) -> Result<()> {
        let tx = self
            .inner
            .begin_write()
            .wrap_err("set_hidden: begin_write")?;
        let new_bytes = {
            let t = tx.open_table(TASKS).wrap_err("set_hidden: open table")?;
            match t.get(&id).wrap_err("set_hidden: read")? {
                Some(v) => {
                    let mut task: Task = codec::decode(v.value())?;
                    if task.hidden == hidden {
                        None
                    } else {
                        task.hidden = hidden;
                        Some(codec::encode(&task)?)
                    }
                }
                None => None,
            }
        };
        if let Some(bytes) = new_bytes {
            let mut t = tx.open_table(TASKS).wrap_err("set_hidden: reopen table")?;
            t.insert(&id, bytes.as_slice())
                .wrap_err("set_hidden: insert")?;
        }
        tx.commit().wrap_err("set_hidden: commit")?;
        Ok(())
    }

    /// Persists `run` against `task_id`, assigning it the next sequence
    /// number for that task and evicting older runs once the per-task
    /// cap is exceeded. The `seq` field of the input is overwritten;
    /// `created_at` is preserved.
    pub fn add_run(&self, mut run: Run) -> Result<Run> {
        let tx = self.inner.begin_write().wrap_err("add_run: begin_write")?;
        {
            let mut t = tx.open_table(RUNS).wrap_err("add_run: open table")?;

            let lo = run_key(run.task_id, 0);
            let hi = run_key(run.task_id, u64::MAX);
            let existing_keys: Vec<[u8; 24]> = t
                .range::<&[u8; 24]>(&lo..=&hi)
                .wrap_err("add_run: range")?
                .filter_map(|e| e.ok().map(|(k, _)| *k.value()))
                .collect();

            let next_seq = existing_keys
                .last()
                .map(|k| {
                    let mut buf = [0u8; 8];
                    buf.copy_from_slice(&k[16..]);
                    u64::from_be_bytes(buf) + 1
                })
                .unwrap_or(0);
            run.seq = next_seq;

            let key = run_key(run.task_id, run.seq);
            let bytes = codec::encode(&run)?;
            t.insert(&key, bytes.as_slice())
                .wrap_err("add_run: insert")?;

            let total = existing_keys.len() + 1;
            if total > RUNS_PER_TASK_LIMIT {
                for old in &existing_keys[..total - RUNS_PER_TASK_LIMIT] {
                    t.remove(old).wrap_err("add_run: evict")?;
                }
            }
        }
        tx.commit().wrap_err("add_run: commit")?;
        Ok(run)
    }

    /// All runs for `task_id`, newest first. Empty if the task has none.
    pub fn runs_of(&self, task_id: TaskId) -> Result<Vec<Run>> {
        let tx = self.inner.begin_read().wrap_err("runs_of: begin_read")?;
        let t = tx.open_table(RUNS).wrap_err("runs_of: open table")?;
        let lo = run_key(task_id, 0);
        let hi = run_key(task_id, u64::MAX);
        let mut out = Vec::new();
        for entry in t.range::<&[u8; 24]>(&lo..=&hi).wrap_err("runs_of: range")? {
            let (_, v) = entry.wrap_err("runs_of: row")?;
            out.push(codec::decode::<Run>(v.value())?);
        }
        out.sort_by_key(|r| std::cmp::Reverse(r.seq));
        Ok(out)
    }

    pub fn last_run(&self, task_id: TaskId) -> Result<Option<Run>> {
        Ok(self.runs_of(task_id)?.into_iter().next())
    }

    pub fn task_count(&self) -> Result<u64> {
        let tx = self.inner.begin_read().wrap_err("task_count: begin_read")?;
        let t = tx.open_table(TASKS).wrap_err("task_count: open table")?;
        t.len().wrap_err("task_count: len")
    }
}

fn run_key(task_id: TaskId, seq: u64) -> [u8; 24] {
    let mut k = [0u8; 24];
    k[..16].copy_from_slice(&task_id);
    k[16..].copy_from_slice(&seq.to_be_bytes());
    k
}

#[cfg(test)]
mod tests {
    use solver::term::TermBuf;
    use tempfile::TempDir;

    use crate::{
        id::compute_task_id,
        run::{Run, RunStats},
        trace::{SolutionTrace, TraceInference, TraceStatus, TraceTerm},
    };

    use super::*;

    fn fresh_db() -> (Db, TempDir) {
        let dir = TempDir::new().unwrap();
        let db = Db::open(dir.path().join("db.redb")).unwrap();
        (db, dir)
    }

    fn sample_task() -> Task {
        let givens = vec![
            TermBuf::symbol(">")
                .arg(TermBuf::variable("x"))
                .arg(TermBuf::number(0)),
        ];
        let goal = TermBuf::symbol("find").arg(TermBuf::variable("x"));
        let id = compute_task_id(&givens, &goal);
        Task {
            id,
            name: "sample".into(),
            text: "x>0; find(x)".into(),
            group: "test".into(),
            givens,
            goal,
            possible_answers: vec![],
            hidden: false,
            created_at: 0,
        }
    }

    fn sample_run(task_id: TaskId) -> Run {
        Run {
            task_id,
            seq: 0,
            created_at: 0,
            stats: RunStats {
                cycles:      42,
                status:      TraceStatus::NotDone,
                answer:      None,
                duration_ms: None,
            },
            solution: SolutionTrace {
                status:        TraceStatus::NotDone,
                terms:         vec![TraceTerm {
                    term:      TermBuf::variable("x"),
                    inference: TraceInference::Condition,
                }],
                sub_solutions: vec![],
            },
        }
    }

    #[test]
    fn task_put_get_idempotent() {
        let (db, _tmp) = fresh_db();
        let task = sample_task();
        db.put_task(&task).unwrap();
        db.put_task(&task).unwrap();
        let back = db.get_task(task.id).unwrap().unwrap();
        assert_eq!(back.id, task.id);
        assert_eq!(back.group, "test");
        assert_eq!(db.task_count().unwrap(), 1);
    }

    #[test]
    fn run_seq_increments_and_caps_at_limit() {
        let (db, _tmp) = fresh_db();
        let task = sample_task();
        db.put_task(&task).unwrap();

        for _ in 0..(RUNS_PER_TASK_LIMIT + 5) {
            db.add_run(sample_run(task.id)).unwrap();
        }
        let runs = db.runs_of(task.id).unwrap();
        assert_eq!(runs.len(), RUNS_PER_TASK_LIMIT);
        // Newest first, newest seq = total inserts - 1 = LIMIT + 4.
        assert_eq!(
            runs.first().unwrap().seq,
            (RUNS_PER_TASK_LIMIT + 5 - 1) as u64
        );
        // Oldest retained seq = total - LIMIT = 5.
        assert_eq!(runs.last().unwrap().seq, 5);
    }

    #[test]
    fn hidden_flag_toggles() {
        let (db, _tmp) = fresh_db();
        let task = sample_task();
        db.put_task(&task).unwrap();
        db.set_hidden(task.id, true).unwrap();
        assert!(db.get_task(task.id).unwrap().unwrap().hidden);
        db.set_hidden(task.id, false).unwrap();
        assert!(!db.get_task(task.id).unwrap().unwrap().hidden);
    }

    #[test]
    fn remove_task_drops_runs() {
        let (db, _tmp) = fresh_db();
        let task = sample_task();
        db.put_task(&task).unwrap();
        db.add_run(sample_run(task.id)).unwrap();
        db.add_run(sample_run(task.id)).unwrap();
        db.remove_task(task.id).unwrap();
        assert!(db.get_task(task.id).unwrap().is_none());
        assert!(db.runs_of(task.id).unwrap().is_empty());
    }

    #[test]
    fn reopen_same_version_succeeds() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("db.redb");
        {
            let db = Db::open(&path).unwrap();
            db.put_task(&sample_task()).unwrap();
        }
        // Reopening a version-stamped db must succeed and keep the data.
        let db = Db::open(&path).unwrap();
        assert_eq!(db.task_count().unwrap(), 1);
    }

    #[test]
    fn open_rejects_version_mismatch() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("db.redb");
        {
            let db = Db::open(&path).unwrap();
            db.put_task(&sample_task()).unwrap();
        }
        // Stamp a foreign version directly, then Db::open must refuse.
        {
            let raw = Database::create(&path).unwrap();
            let tx = raw.begin_write().unwrap();
            {
                let mut meta = tx.open_table(META).unwrap();
                meta.insert(SCHEMA_KEY, SCHEMA_VERSION + 1).unwrap();
            }
            tx.commit().unwrap();
        }
        assert!(Db::open(&path).is_err());
    }

    #[test]
    fn open_rejects_unversioned_db_with_data() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("db.redb");
        // Simulate a pre-versioning db: a task row but no meta table.
        {
            let raw = Database::create(&path).unwrap();
            let tx = raw.begin_write().unwrap();
            {
                let mut t = tx.open_table(TASKS).unwrap();
                let task = sample_task();
                t.insert(&task.id, codec::encode(&task).unwrap().as_slice())
                    .unwrap();
            }
            tx.commit().unwrap();
        }
        assert!(Db::open(&path).is_err());
    }

    #[test]
    fn last_run_returns_newest() {
        let (db, _tmp) = fresh_db();
        let task = sample_task();
        db.put_task(&task).unwrap();
        let mut last_seq = 0;
        for _ in 0..3 {
            last_seq = db.add_run(sample_run(task.id)).unwrap().seq;
        }
        assert_eq!(db.last_run(task.id).unwrap().unwrap().seq, last_seq);
    }
}
