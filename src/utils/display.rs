use std::fmt;

// use itertools::Itertools;

pub struct VecDisplay<'a, T: fmt::Display>(pub &'a Vec<T>);

impl<'a, T: fmt::Display> fmt::Display for VecDisplay<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|s| format!("{}", s))
                .intersperse(", ".to_string())
                .collect::<String>()
        )?;

        write!(f, "]")
    }
}
