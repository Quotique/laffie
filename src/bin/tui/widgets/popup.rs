use ratatui::{
    prelude::*,
    text::Line,
    widgets::{Block, Clear, Padding},
};

use crate::theme::Theme;

#[derive(Debug)]
pub struct Popup<'a> {
    title: Line<'a>,
    theme: &'a Theme,
}

impl<'a> Popup<'a> {
    pub fn new(title: Line<'a>, theme: &'a Theme) -> Self {
        Self { title, theme }
    }

    pub fn inner(&self, area: Rect) -> Rect {
        self.block().inner(Self::area(area))
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let popup_area = Self::area(area);
        let block = self.block();

        Clear.render(popup_area, frame.buffer_mut());
        frame.render_widget(&block, popup_area);
    }

    fn block<'b>(&'b self) -> Block<'b> {
        self.theme
            .block(true, self.title.clone())
            .padding(Padding::uniform(2))
    }

    fn area(area: Rect) -> Rect {
        Rect {
            x:      area.width / 4,
            y:      area.height / 3,
            width:  area.width / 2,
            height: area.height / 3,
        }
    }
}
