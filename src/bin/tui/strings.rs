// All user-visible TUI strings live here so a future translation can swap
// them in one place. Domain labels rendered inside Tracing/Solution windows
// (e.g. "Task: ", "Answer: ") are kept next to their renderers — they read
// closer to data than to chrome.

pub mod pane_title {
    pub const RULES: &str = "Rules";
    pub const TASKS: &str = "Tasks";
    pub const TRACING: &str = "Tracing";
    pub const DETAILED: &str = "Detailed";
    pub const SETTINGS: &str = "Settings";
}

pub mod popup_title {
    pub const ERROR: &str = "Error";
    pub const HELP: &str = "Help";
}

pub mod error {
    pub const RELOAD_FAILED: &str = "Reload failed";
    pub const RELOAD_ALL_FAILED: &str = "Reload all failed";
    pub const SAVE_SETTINGS_FAILED: &str = "Save settings failed";
    pub const SOLVER_THREAD_CRASH: &str = "Solver thread crashed";
}

pub mod status_badge {
    pub const WRONG: &str = "[wrong]   ";
    pub const UNSOLVED: &str = "[unsolved]";
    pub const SOLVED: &str = "[solved]  ";
    pub const NOT_STARTED: &str = "[idle]    ";
}

pub mod solution {
    pub const PRESS_S_TO_SOLVE: &str = "Press s to solve";
    pub const DIFF_IDENTICAL: &str = "  identical";
    pub const NO_ANSWER: &str = "no answer";

    pub fn diff_header(prev: usize, cur: usize) -> String {
        format!("Diff vs previous ({prev} → {cur} steps):")
    }

    pub fn problem_tasks_header(count: usize) -> String {
        format!("Problem tasks ({count}):")
    }
}

pub mod directory_summary {
    pub const GROUP: &str = "Group: ";
    pub const TOTAL: &str = "Total: ";
    pub const NOT_STARTED: &str = "Not started: ";
    pub const SOLVED: &str = "Solved: ";
    pub const NOT_SOLVED: &str = "Not solved: ";
    pub const WRONG_ANSWERS: &str = "Wrong answers: ";
}

pub mod help {
    pub const ENTRIES: &[(&str, &str)] = &[
        ("F1 / F2 / F3 / F4", "switch tab"),
        ("Tab / Shift+Tab", "next / previous panel"),
        ("← ↑ → ↓", "navigation"),
        ("PgUp / PgDn", "page up / down"),
        ("Ctrl+u / Ctrl+d", "page up / down"),
        ("Home / End", "jump to top / bottom"),
        ("Space / Enter", "toggle (tree node, etc.)"),
        ("/", "filter (Rules)"),
        ("s", "solve selected task"),
        ("a", "solve all tasks"),
        ("c", "cancel running solver"),
        ("r", "reload rules"),
        ("Shift+R", "reload rules + tasks"),
        ("e", "open selected source in $EDITOR"),
        ("Ctrl+S", "save settings (Settings tab)"),
        ("?", "toggle help"),
        ("Esc", "dismiss popup / cancel filter"),
        ("q", "quit"),
    ];
}
