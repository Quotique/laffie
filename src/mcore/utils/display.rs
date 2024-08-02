use std::fmt;

pub struct VecDisplay<'a, T: fmt::Display>(pub &'a Vec<T>);

impl<'a, T: fmt::Display> fmt::Display for VecDisplay<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut iter = self.0.iter().peekable();
        write!(f, "[")?;
        while let Some(s) = iter.next() {
            write!(f, "{}{}", s, if iter.peek().is_some() { ", " } else { "" })?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_display_test() {
        let test = vec![1, 2, 3];

        insta::assert_snapshot!(VecDisplay(&test).to_string(), @"");
    }
}
