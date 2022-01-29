use std::env;

const WHO_IS_STR: &str =
    r#"Laffie - пользовательский интерфейс к ядру символьной арифметики Minerva."#;

pub struct Static;

impl Static {
    pub fn start() -> String {
        format!("{}\n\n{}", WHO_IS_STR, Self::version())
    }

    pub fn version() -> String {
        format!(
            "Laffie: v{}\nMinerva: v{}",
            env!("CARGO_PKG_VERSION"),
            mcore::version_str()
        )
    }
}
