use std::fmt::Write;

use eyre::{Result, bail};

use solver::term::TermBuf;

/// Stable, content-addressed identifier of a [`Task`](crate::Task).
///
/// 16 bytes of `blake3` over a domain-separated, canonical encoding of the
/// task's givens (as a multiset) and goal. Truncating to 128 bits keeps the
/// keys short while leaving collision probability astronomically low for the
/// expected scale (~10^5 tasks).
pub type TaskId = [u8; 16];

const DOMAIN: &[u8] = b"laffie:task:v1";
const GOAL_SEP: &[u8] = b"|goal|";

/// Compute the [`TaskId`] for a task with the given conditions and goal.
///
/// Givens are hashed as a multiset: each element is JSON-encoded individually
/// and the resulting byte strings are sorted before being fed into the hasher.
/// Reordering the input slice does not change the id.
pub fn compute_task_id(givens: &[TermBuf], goal: &TermBuf) -> TaskId {
    let mut sorted_givens: Vec<Vec<u8>> = givens
        .iter()
        .map(|t| serde_json::to_vec(t).expect("TermBuf serialization is infallible"))
        .collect();
    sorted_givens.sort();

    let mut hasher = blake3::Hasher::new();
    hasher.update(DOMAIN);
    for g in &sorted_givens {
        hasher.update(&(g.len() as u32).to_le_bytes());
        hasher.update(g);
    }
    hasher.update(GOAL_SEP);
    let goal_bytes = serde_json::to_vec(goal).expect("TermBuf serialization is infallible");
    hasher.update(&goal_bytes);

    let mut id = [0u8; 16];
    id.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    id
}

/// Lower-case 32-character hexadecimal rendering of a [`TaskId`].
pub fn id_to_hex(id: &TaskId) -> String {
    let mut s = String::with_capacity(32);
    for b in id {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Parses a 32-character hexadecimal [`TaskId`].
pub fn id_from_hex(s: &str) -> Result<TaskId> {
    if s.len() != 32 {
        bail!("expected 32 hex chars, got {}", s.len());
    }
    let mut id = [0u8; 16];
    for (i, byte) in id.iter_mut().enumerate() {
        let hi = hex_digit(s.as_bytes()[i * 2])?;
        let lo = hex_digit(s.as_bytes()[i * 2 + 1])?;
        *byte = (hi << 4) | lo;
    }
    Ok(id)
}

fn hex_digit(b: u8) -> Result<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => bail!("not a hex digit: {:?}", b as char),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gt_zero(name: &str) -> TermBuf {
        TermBuf::symbol(">")
            .arg(TermBuf::variable(name))
            .arg(TermBuf::number(0))
    }

    fn lt_zero(name: &str) -> TermBuf {
        TermBuf::symbol("<")
            .arg(TermBuf::variable(name))
            .arg(TermBuf::number(0))
    }

    fn find(name: &str) -> TermBuf {
        TermBuf::symbol("find").arg(TermBuf::variable(name))
    }

    #[test]
    fn givens_order_is_irrelevant() {
        let g1 = gt_zero("x");
        let g2 = gt_zero("y");
        let goal = find("x");

        let id_a = compute_task_id(&[g1.clone(), g2.clone()], &goal);
        let id_b = compute_task_id(&[g2, g1], &goal);
        assert_eq!(id_a, id_b);
    }

    #[test]
    fn different_goal_yields_different_id() {
        let givens = vec![gt_zero("x")];
        let id_a = compute_task_id(&givens, &find("x"));
        let id_b = compute_task_id(&givens, &find("y"));
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn different_givens_yield_different_id() {
        let goal = find("x");
        let id_a = compute_task_id(&[gt_zero("x")], &goal);
        let id_b = compute_task_id(&[lt_zero("x")], &goal);
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn empty_givens_does_not_panic() {
        let id = compute_task_id(&[], &find("x"));
        assert_ne!(id, [0u8; 16]);
    }

    #[test]
    fn hex_round_trip() {
        let id = compute_task_id(&[gt_zero("x")], &find("x"));
        let hex = id_to_hex(&id);
        assert_eq!(hex.len(), 32);
        assert_eq!(id_from_hex(&hex).unwrap(), id);
    }

    #[test]
    fn hex_uppercase_is_accepted() {
        let id = compute_task_id(&[gt_zero("x")], &find("x"));
        let hex = id_to_hex(&id).to_uppercase();
        assert_eq!(id_from_hex(&hex).unwrap(), id);
    }

    #[test]
    fn hex_rejects_wrong_length() {
        assert!(id_from_hex("deadbeef").is_err());
        assert!(id_from_hex(&"f".repeat(33)).is_err());
    }

    #[test]
    fn hex_rejects_non_hex_chars() {
        assert!(id_from_hex(&"g".repeat(32)).is_err());
    }
}
