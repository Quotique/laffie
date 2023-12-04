use std::sync::Arc;

use parking_lot::RwLock;

#[derive(Clone, Debug)]
pub struct CalculationLimits {
    limit:  usize,
    wasted: Arc<RwLock<usize>>,
}

impl CalculationLimits {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            wasted: Arc::new(RwLock::new(0)),
        }
    }

    pub fn partial_sublimit(&self, part: f64) -> Self {
        if part >= 1.0 {
            panic!("bad part size");
        }
        let wasted = self.current_wasted();

        Self {
            limit:  wasted + ((self.limit - wasted) as f64 * part) as usize,
            wasted: self.wasted.clone(),
        }
    }

    pub fn take(&self, value: usize) -> bool {
        let mut wasted = self.wasted.write();
        if (self.limit - *wasted) < value {
            false
        } else {
            *wasted += value;
            true
        }
    }

    #[inline]
    fn current_wasted(&self) -> usize {
        *self.wasted.read()
    }
}
