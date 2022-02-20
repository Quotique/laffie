use std::str::FromStr;

use bincode::{Decode, Encode};
use trees::tr;

use super::{
    term::{CompactString, Decimal, StatementNode, StatementTree, Term},
    Statement,
};

impl Encode for Term {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        match self {
            Term::Symbol(s) => {
                Encode::encode(&1i8, encoder)?;
                Encode::encode(&s, encoder)?;
            }
            Term::Param(p) => {
                Encode::encode(&2i8, encoder)?;
                Encode::encode(&p.as_ref().as_str(), encoder)?;
            }
            Term::Variable(v) => {
                Encode::encode(&3i8, encoder)?;
                Encode::encode(&v.as_ref().as_str(), encoder)?;
            }
            Term::Number(d) => {
                Encode::encode(&4i8, encoder)?;
                Encode::encode(&d.to_string().as_str(), encoder)?;
            }
            Term::Placeholder(p) => {
                Encode::encode(&5i8, encoder)?;
                Encode::encode(&u64::from(*p), encoder)?;
            }
        }
        Ok(())
    }
}

impl Decode for Term {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let index: i8 = Decode::decode(decoder)?;
        let term = match index {
            1 => Term::Symbol(Decode::decode(decoder)?),
            2 => {
                let s: String = Decode::decode(decoder)?;
                Term::Param(CompactString::from(s).into())
            }
            3 => {
                let s: String = Decode::decode(decoder)?;
                Term::Variable(CompactString::from(s).into())
            }
            4 => {
                let s: String = Decode::decode(decoder)?;
                // TODO: remove unwrap
                Term::Number(Decimal::from_str(&s).unwrap())
            }
            5 => {
                let p: u64 = Decode::decode(decoder)?;
                Term::Placeholder(p.into())
            }
            _ => unreachable!(),
        };
        Ok(term)
    }
}

impl Decode for Statement {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let mut tree_vec: Vec<(isize, StatementTree)> = vec![];
        loop {
            let parent_no: isize = Decode::decode(decoder)?;
            if parent_no == -2 {
                break;
            }

            let data: Term = Decode::decode(decoder)?;
            tree_vec.push((parent_no, tr(data)));
        }

        let mut statement_node = None;
        while let Some((parent_no, node)) = tree_vec.pop() {
            if parent_no == -1 {
                assert!(tree_vec.is_empty(), "root element is not last");
                statement_node = Some(node);
            } else {
                tree_vec[parent_no as usize].1.push_front(node);
            }
        }
        let tree = statement_node.take().unwrap();

        Ok(Statement {
            tree,
            // TODO: encode/decode for binds
            binds: Default::default(),
        })
    }
}

impl Encode for Statement {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        let mut last_no = -1;
        encode_node(-1, &mut last_no, self.root(), encoder)?;
        Encode::encode(&-2, encoder)?;
        assert!(self.binds.is_empty(), "binds encode is not supported");
        Ok(())
    }
}

fn encode_node<E: bincode::enc::Encoder>(
    parent_no: isize,
    last_no: &mut isize,
    node: &StatementNode,
    encoder: &mut E,
) -> core::result::Result<(), bincode::error::EncodeError> {
    Encode::encode(&parent_no, encoder)?;
    Encode::encode(node.data(), encoder)?;

    *last_no += 1;
    let node_no = *last_no;

    for i in node.iter() {
        encode_node(node_no, last_no, i, encoder)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use bincode::config;

    use crate::statement::{
        statement_with_params,
        term::{CompactString, Decimal, Term},
        Statement,
    };

    #[test]
    fn term_codec_test() {
        for t in &[
            Term::Symbol(2),
            Term::Param(CompactString::from("a").into()),
            Term::Variable(CompactString::from("x").into()),
            Term::Number(Decimal::from(100)),
            Term::Placeholder(10.into()),
        ] {
            let config = config::standard();

            let encoded: Vec<u8> = bincode::encode_to_vec(t, config).unwrap();
            let (decoded, len): (Term, usize) =
                bincode::decode_from_slice(&encoded[..], config).unwrap();

            assert_eq!(t, &decoded);
            assert_eq!(len, encoded.len()); // read all bytes
        }
    }

    #[test]
    fn statement_encode_test() {
        let statement = statement_with_params("a*x+b==0 => x==-b/a");
        let config = config::standard();

        let encoded: Vec<u8> = bincode::encode_to_vec(&statement, config).unwrap();
        let (decoded, len): (Statement, usize) =
            bincode::decode_from_slice(&encoded[..], config).unwrap();

        assert_eq!(statement, decoded);
        assert_eq!(len, encoded.len());
    }
}
