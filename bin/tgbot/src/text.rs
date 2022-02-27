use std::env;

use chrono::{offset::Utc, DateTime};

use database::ProblemRecord;

const WHO_IS_STR: &str =
    r#"Laffie - пользовательский интерфейс к ядру символьной арифметики Minerva."#;

const WARNING: &str = r#"
<u><b>Внимание:</b></u> бот находится в тестовом режиме.
Работоспособность не гарантируется."#;

pub struct Text;

impl Text {
    pub fn start() -> String {
        format!(
            "{}{}\n\n{}\n\n{}",
            WHO_IS_STR,
            WARNING,
            Self::version(),
            Self::system()
        )
    }

    pub fn system() -> String {
        format!(
            "{} {} {} {}MHz",
            sys_info::os_type().unwrap(),
            sys_info::os_release().unwrap(),
            env::consts::ARCH,
            cpu_freq::get()[0].max.unwrap(),
        )
    }

    pub fn version() -> String {
        format!(
            "Laffie: v{}\nMinerva: v{}",
            env!("CARGO_PKG_VERSION"),
            mcore::version_str()
        )
    }

    pub fn problem_text(problem: &ProblemRecord) -> String {
        let answer = problem
            .runs
            .last()
            .map(|x| match &x.status {
                Ok(s) => format!(
                    "\n<i>Ответ:</i> {} [получен: {}, циклов сканирования: {}]\n",
                    s,
                    DateTime::<Utc>::from(x.timestamp).format("%Y-%m-%d %T"),
                    x.cycles_count
                ),
                Err(_) => "<i>Задача не решена</i>\n".to_owned(),
            })
            .unwrap_or("".to_owned());
        format!(
            "<u><i>Задача</i></u> 0x{:x}\n<i>цель:</i> {}\n<i>условия:</i>\n  {}\n{}",
            problem.id,
            problem.target,
            problem
                .conditions
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join("\n  "),
            answer
        )
    }

    pub fn empty_problems_list() -> String {
        "Нет задач".to_string()
    }
}
