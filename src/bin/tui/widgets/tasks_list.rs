use itertools::Itertools;
use ratatui::{prelude::*, widgets::StatefulWidget};
use trees::{Node, Tree};
use tui_tree_widget::{Tree as TuiTree, TreeItem, TreeState};

use solver::{task::SolutionStatus, CompactString};
use utils::{IndexedTree, TreeIndex};

use crate::{state::TaskState, theme::Theme};

pub struct TasksList<'a> {
    pub tasks_index: &'a Tree<TasksNode>,
    pub tasks:       &'a [TaskState],
}

#[derive(Clone, Debug)]
pub enum TasksNode {
    Task(usize),
    Directory(DirectoryStatus),
}

#[derive(Debug, Clone)]
pub struct DirectoryStatus {
    pub dir_name:           CompactString,
    pub solved_count:       usize,
    pub unsolved_count:     usize,
    pub wrong_answer_count: usize,
    pub not_started_count:  usize,
}

impl DirectoryStatus {
    pub fn total(&self) -> usize {
        self.solved_count + self.unsolved_count + self.wrong_answer_count + self.not_started_count
    }
}

impl From<CompactString> for DirectoryStatus {
    fn from(dir_name: CompactString) -> Self {
        Self {
            dir_name,
            solved_count: 0,
            unsolved_count: 0,
            wrong_answer_count: 0,
            not_started_count: 0,
        }
    }
}

impl TasksNode {
    pub fn new_task(task_pos: usize) -> Self {
        Self::Task(task_pos)
    }

    pub fn new_directory(dir_name: CompactString) -> Self {
        Self::Directory(dir_name.into())
    }
}

impl<'a> StatefulWidget for TasksList<'a> {
    type State = TreeState<TreeIndex>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let items = [self.tree(self.tasks_index.root())];

        let widget = TuiTree::new(&items)
            .expect("all item identifiers are unique")
            .experimental_scrollbar(Some(self.theme().scrollbar()))
            .highlight_style(self.theme().tree_cursor_style())
            .highlight_symbol(">");

        <TuiTree<TreeIndex> as StatefulWidget>::render(widget, area, buf, state);
    }
}

impl<'a> TasksList<'a> {
    fn tree(&self, tasks_node: &Node<TasksNode>) -> TreeItem<'static, TreeIndex> {
        let wrong_answer = self.theme().wrong_answer();
        let unsolved = self.theme().unsolved();
        let solved = self.theme().solved();
        let not_started = self.theme().not_started();
        let default = self.theme().default();

        let text = match tasks_node.data() {
            TasksNode::Task(task) => {
                let task = &self.tasks[*task];
                let line_style = match task.solution.status {
                    SolutionStatus::NotDone => not_started,
                    SolutionStatus::Err(_) => unsolved,
                    SolutionStatus::Answer(_) if task.solution.validate_answer() => solved,
                    _ => wrong_answer,
                };

                let conditions =
                    format!("[{}]", task.solution.task.conditions.iter().format(", "),);
                Line::from(vec![
                    Span::styled(task.solution.task.purpose.to_string(), line_style),
                    Span::from(" "),
                    Span::styled(conditions, default),
                ])
            }
            TasksNode::Directory(dir) => Line::from(vec![
                Span::styled(dir.dir_name.to_string(), self.theme().highlighted()),
                Span::from("[".to_owned()),
                Span::styled(format!("{}", dir.not_started_count), not_started),
                Span::styled(format!(" {}", dir.solved_count), solved),
                Span::styled(format!(" {}", dir.unsolved_count), unsolved),
                Span::styled(format!(" {}", dir.wrong_answer_count), wrong_answer),
                Span::from("]".to_owned()),
            ]),
        };

        let children: Vec<_> = tasks_node.iter().map(|s| self.tree(s)).collect();
        if children.is_empty() {
            TreeItem::new_leaf(tasks_node.id(), text)
        } else {
            TreeItem::new(tasks_node.id(), text, children).unwrap()
        }
    }

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}
