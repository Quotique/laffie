use std::{collections::HashMap, str::FromStr};

use serde::{de, Deserialize as _};
use serde_derive::Deserialize;
use slog::Level;

#[derive(Debug, Deserialize)]
pub struct ConfLevel(#[serde(deserialize_with = "deserialize_level")] pub Level);

#[derive(Debug, Deserialize)]
pub struct Config {
    pub filename:          String,
    pub level:             ConfLevel,
    pub num_files:         usize,
    pub file_rotate_bytes: usize,
    pub target_levels:     HashMap<String, ConfLevel>,
}

fn deserialize_level<'de, D>(deserializer: D) -> Result<Level, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = <String>::deserialize(deserializer)?;
    Level::from_str(s.as_str()).map_err(|_| {
        de::Error::invalid_value(
            de::Unexpected::Str(s.as_str()),
            &"[Error, Warn, Into, Debug, Trace]",
        )
    })
}
