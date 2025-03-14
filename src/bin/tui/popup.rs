use ratatui::{
    prelude::*,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

#[derive(Debug)]
pub struct Popup<'a> {
    title: Line<'a>,
    text:  Paragraph<'a>,
}

impl Popup<'_> {
    pub fn new(title: Line<'static>, text: Paragraph<'static>) -> Self {
        Self { title, text }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let popup_area = Rect {
            x:      area.width / 4,
            y:      area.height / 3,
            width:  area.width / 2,
            height: area.height / 3,
        };
        Clear.render(popup_area, frame.buffer_mut());
        let bad_popup = self
            .text
            .clone()
            .wrap(Wrap { trim: true })
            .style(Style::new().white())
            .block(
                Block::new()
                    .title(self.title.clone())
                    .title_style(Style::new().red().bold())
                    .borders(Borders::ALL)
                    .border_style(Style::new().gray()),
            );
        frame.render_widget(bad_popup, popup_area);
    }
}
