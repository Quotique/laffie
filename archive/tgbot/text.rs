use std::env;

use database::TaskRecord;
use itertools::Itertools;

pub struct Text;

#[cfg(target_os = "linux")]
fn get_cpu_freq() -> f32 {
    cpu_freq::get()[0].max.unwrap()
}

#[cfg(not(target_os = "linux"))]
fn get_cpu_freq() -> f32 {
    f32::NAN
}

impl Text {
    pub fn system() -> String {
        format!(
            "{} {} {} {}MHz",
            sys_info::os_type().unwrap(),
            sys_info::os_release().unwrap(),
            env::consts::ARCH,
            get_cpu_freq()
        )
    }

    pub fn version() -> String {
        format!(
            "Laffie: v{}\nMinerva: v{}",
            env!("CARGO_PKG_VERSION"),
            solver::version_str()
        )
    }

    pub fn task_text(task: &TaskRecord) -> String {
        let answer = task
            .runs
            .last()
            .map(|x| {
                // TODO: no answer, i18n
                format!(
                    "\n<i>Возможные ответы:</i> [{}] [циклов сканирования: {x}]\n",
                    task.answer.iter().format(", "),
                )
            })
            .unwrap_or_else(|| "".to_owned());
        format!(
            "<u><i>Задача</i></u> 0x{:x}\n<i>цель:</i> {}\n<i>условия:</i>\n  {}\n{}",
            task.id,
            task.goal,
            task.givens
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join("\n  "),
            answer
        )
    }
}
