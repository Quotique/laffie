use std::{collections::HashMap, iter::FromIterator, sync::RwLock};

use patricia_tree::StringPatriciaMap as PrefixMap;
use slog::Level;

#[derive(Debug)]
pub struct Filter {
    max_level: Level,
    prefixes:  PrefixMap<Level>,
    cache:     RwLock<HashMap<String, Level>>,
}

impl Filter {
    pub fn new(max_level: Level, levels: impl Iterator<Item = (String, Level)>) -> Self {
        Self {
            max_level,
            prefixes: PrefixMap::from_iter(levels),
            cache: Default::default(),
        }
    }

    pub fn filter(&self, module: impl AsRef<str>, level: Level) -> bool {
        let module = module.as_ref();
        if let Some(cached) = self.cache.read().unwrap().get(module) {
            return level <= *cached;
        }

        let newlvl = *self
            .prefixes
            .get_longest_common_prefix(module)
            .map(|x| x.1)
            .unwrap_or(&self.max_level);
        let _ = self
            .cache
            .try_write()
            .map(|mut x| x.insert(module.to_owned(), newlvl));
        level <= newlvl
    }
}
