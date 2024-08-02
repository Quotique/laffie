use std::fmt;

pub struct Paginator {
    pages:           Vec<String>,
    page_size_limit: usize,
}

impl Paginator {
    pub fn new(page_size_limit: usize) -> Self {
        Paginator {
            pages: vec![String::default()],
            page_size_limit,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &'_ String> {
        self.pages.iter()
    }
}

impl fmt::Write for Paginator {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.len() > self.page_size_limit {
            return Err(fmt::Error);
        }

        if self.pages.last().unwrap().len() + s.len() > self.page_size_limit {
            self.pages.push(String::default());
        }
        self.pages.last_mut().unwrap().write_str(s)
    }
}
