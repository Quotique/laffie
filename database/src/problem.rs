use std::{
    convert::{From, Into},
    path::Path,
    sync::Arc,
};

use bincode::{config, config::Configuration, Decode, Encode};
use rocksdb::{Error, DB};

use mcore::{
    problem::Problem,
    statement::{MarkedStatement, Statement},
};

#[derive(Encode, Decode)]
pub struct ProblemRecord {
    pub id:         u128,
    pub conditions: Vec<Statement>,
    pub target:     Statement,
}

pub struct ProblemDb {
    db:     DB,
    config: Configuration,
}

impl From<&Problem> for ProblemRecord {
    fn from(value: &Problem) -> Self {
        Self {
            id:         value.id as u128,
            conditions: Vec::from_iter(value.conditions.iter().map(|x| (*x.statement).clone())),
            target:     (*value.target.statement).clone(),
        }
    }
}

impl Into<Problem> for ProblemRecord {
    fn into(self) -> Problem {
        Problem {
            id:               self.id as u64,
            conditions:       Vec::from_iter(
                self.conditions
                    .into_iter()
                    .map(|x| MarkedStatement::from(Arc::new(x))),
            ),
            target:           MarkedStatement::from(Arc::new(self.target)),
            subproblem_level: 0,
        }
    }
}

impl ProblemRecord {
    fn is_same(&self, other: &Self) -> bool {
        if self.target != other.target {
            return false;
        }
        for i in self.conditions.iter() {
            if !other.conditions.iter().any(|x| x == i) {
                return false;
            }
        }
        for i in other.conditions.iter() {
            if !self.conditions.iter().any(|x| x == i) {
                return false;
            }
        }
        true
    }
}

impl ProblemDb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Self {
            db:     DB::open_default(path)?,
            config: config::standard(),
        })
    }

    pub fn get(&self, id: u128) -> Result<Option<ProblemRecord>, Error> {
        let key = id.to_le_bytes();
        Ok(self.db.get(key)?.map(|b| {
            let (decoded, _): (ProblemRecord, usize) =
                bincode::decode_from_slice(&b[..], self.config).unwrap();
            decoded
        }))
    }

    pub fn get_or_insert(&self, mut problem: ProblemRecord) -> Result<ProblemRecord, Error> {
        let (_, problem_id) = split_id(problem.id);

        for number in 1..u64::MAX {
            let id = compose_id(number, problem_id);
            match self.get(id)? {
                Some(p) => {
                    if p.is_same(&problem) {
                        return Ok(p);
                    }
                }
                None => {
                    problem.id = id;
                    self.put(&problem)?;
                    return Ok(problem);
                }
            }
        }
        unreachable!()
    }

    pub fn put(&self, problem: &ProblemRecord) -> Result<(), Error> {
        let key = problem.id.to_le_bytes();
        let encoded: Vec<u8> = bincode::encode_to_vec(problem, self.config).unwrap();

        self.db.put(key, encoded)
    }
}

fn compose_id(number: u64, problem_id: u64) -> u128 {
    ((number as u128) << 64) + problem_id as u128
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
        for (number, problem_id) in &[(0, 0), (1, 1000), (100, 5000)] {
            let id = compose_id(*number, *problem_id);
            let (decoded_number, decoded_problem_id) = split_id(id);
            assert_eq!(decoded_number, *number);
            assert_eq!(decoded_problem_id, *problem_id);
        }
    }
}
