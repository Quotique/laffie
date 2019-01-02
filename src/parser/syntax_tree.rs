use std::fmt::{Debug, Error, Formatter};
use std::io::{self, Write};

// #[derive(Copy, Clone)]
// pub enum TagType {
//     TagIn,
//     TagIs,
// }
//
// pub struct Tag {
//     pub tag_type: TagType,
//     pub value: String,
// }

pub struct Node {
    pub label: String,
    //pub tags: Vec<Tag>,
    pub childs: Vec<Box<Node>>,
}

// impl Debug for Tag {
//     fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
//         match self.tag_type {
//             TagType::TagIn => write!(fmt, "in {}", self.value),
//             TagType::TagIs => write!(fmt, "is {}", self.value),
//         }
//     }
// }

impl Debug for Node {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        write!(
            fmt,
            "Node: {{ label: {} childs: {:?} }}",
            self.label, self.childs
        )
    }
}

impl Node {
    pub fn visual(&self) -> io::Result<()> {
        let mut ident = String::from("");
        self.visual_ident(&mut ident)
    }

    fn visual_ident(&self, ident: &mut String) -> io::Result<()> {
        let ident_len = ident.len();
        let stdout = io::stdout();
        stdout.lock().write(b"\"")?;
        stdout.lock().write(self.label.as_bytes())?;
        stdout.lock().write(b"\"\n")?;
        for (i, item) in self.childs.iter().enumerate() {
            ident.truncate(ident_len);
            stdout.lock().write(ident.as_bytes())?;
            if i != self.childs.len() - 1 {
                ident.push_str("\u{2502} ");
                stdout
                    .lock()
                    .write(String::from("\u{251C}\u{2500}").as_bytes())?;
            } else {
                ident.push_str("  ");
                stdout
                    .lock()
                    .write(String::from("\u{2514}\u{2500}").as_bytes())?;
            }
            item.visual_ident(ident)?;
        }
        ident.truncate(ident_len);
        Ok(())
    }
}
