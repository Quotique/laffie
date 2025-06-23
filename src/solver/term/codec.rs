use std::str::FromStr;

use bincode::{BorrowDecode, Decode, Encode};

use super::{Subterm, SymbolTree, Term, TermNode};
use crate::{CompactString, Decimal};

use trees::tr;

impl Encode for TermNode {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        match self {
            TermNode::Symbol(s) => {
                Encode::encode(&1i8, encoder)?;
                Encode::encode(&s.as_str(), encoder)?;
            }
            TermNode::Param(p) => {
                Encode::encode(&2i8, encoder)?;
                Encode::encode(&p.as_ref().as_str(), encoder)?;
            }
            TermNode::Variable(v) => {
                Encode::encode(&3i8, encoder)?;
                Encode::encode(&v.as_ref().as_str(), encoder)?;
            }
            TermNode::Number(d) => {
                Encode::encode(&4i8, encoder)?;
                Encode::encode(&d.to_string().as_str(), encoder)?;
            }
            TermNode::Placeholder(p) => {
                Encode::encode(&5i8, encoder)?;
                Encode::encode(&u64::from(*p), encoder)?;
            }
        }
        Ok(())
    }
}

impl<Context> Decode<Context> for TermNode {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let index: i8 = Decode::decode(decoder)?;
        let term = match index {
            1 => {
                let s: String = Decode::decode(decoder)?;
                TermNode::with_symbol_opt(&s).ok_or_else(|| {
                    bincode::error::DecodeError::OtherString(format!("bad symbol {s}"))
                })?
            }
            2 => {
                let s: String = Decode::decode(decoder)?;
                TermNode::Param(CompactString::from(s).into())
            }
            3 => {
                let s: String = Decode::decode(decoder)?;
                TermNode::Variable(CompactString::from(s).into())
            }
            4 => {
                let s: String = Decode::decode(decoder)?;
                // TODO: remove unwrap
                TermNode::Number(Decimal::from_str(&s).unwrap())
            }
            5 => {
                let p: u64 = Decode::decode(decoder)?;
                TermNode::Placeholder(p.into())
            }
            _ => unreachable!(),
        };
        Ok(term)
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for TermNode {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let index: i8 = BorrowDecode::borrow_decode(decoder)?;
        let term = match index {
            1 => {
                let s: String = BorrowDecode::borrow_decode(decoder)?;
                TermNode::with_symbol_opt(&s).ok_or_else(|| {
                    bincode::error::DecodeError::OtherString(format!("bad symbol {s}"))
                })?
            }
            2 => {
                let s: String = BorrowDecode::borrow_decode(decoder)?;
                TermNode::Param(CompactString::from(s).into())
            }
            3 => {
                let s: String = BorrowDecode::borrow_decode(decoder)?;
                TermNode::Variable(CompactString::from(s).into())
            }
            4 => {
                let s: String = BorrowDecode::borrow_decode(decoder)?;
                // TODO: remove unwrap
                TermNode::Number(Decimal::from_str(&s).unwrap())
            }
            5 => {
                let p: u64 = BorrowDecode::borrow_decode(decoder)?;
                TermNode::Placeholder(p.into())
            }
            _ => unreachable!(),
        };
        Ok(term)
    }
}

pub fn encode_node<E: bincode::enc::Encoder>(
    parent_no: isize,
    last_no: &mut isize,
    node: Subterm,
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

impl<'de, Context> BorrowDecode<'de, Context> for Term {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let mut tree_vec: Vec<(isize, SymbolTree)> = vec![];
        loop {
            let parent_no: isize = BorrowDecode::borrow_decode(decoder)?;
            if parent_no == -2 {
                break;
            }

            let data: TermNode = BorrowDecode::borrow_decode(decoder)?;
            tree_vec.push((parent_no, tr(data)));
        }

        let mut term_node = None;
        while let Some((parent_no, node)) = tree_vec.pop() {
            if parent_no == -1 {
                assert!(tree_vec.is_empty(), "root element is not last");
                term_node = Some(node);
            } else {
                tree_vec[parent_no as usize].1.push_front(node);
            }
        }
        let tree = term_node.take().unwrap();

        Ok(Term {
            tree,
            // TODO: encode/decode for binds
            binds: Default::default(),
        })
    }
}

impl<Context> Decode<Context> for Term {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let mut tree_vec: Vec<(isize, SymbolTree)> = vec![];
        loop {
            let parent_no: isize = Decode::decode(decoder)?;
            if parent_no == -2 {
                break;
            }

            let data: TermNode = Decode::decode(decoder)?;
            tree_vec.push((parent_no, tr(data)));
        }

        let mut term_node = None;
        while let Some((parent_no, node)) = tree_vec.pop() {
            if parent_no == -1 {
                assert!(tree_vec.is_empty(), "root element is not last");
                term_node = Some(node);
            } else {
                tree_vec[parent_no as usize].1.push_front(node);
            }
        }
        let tree = term_node.take().unwrap();

        Ok(Term {
            tree,
            // TODO: encode/decode for binds
            binds: Default::default(),
        })
    }
}

impl Encode for Term {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        let mut last_no = -1;
        encode_node(-1, &mut last_no, self.as_subterm(), encoder)?;
        Encode::encode(&-2, encoder)?;
        assert!(self.binds.is_empty(), "binds encode is not supported");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bincode::config;

    use crate::{
        term::{term_with_params, Term, TermNode},
        CompactString, Decimal,
    };

    #[test]
    fn symbol_codec_test() {
        for t in &[
            TermNode::with_symbol("+"),
            TermNode::Param(CompactString::from("a").into()),
            TermNode::Variable(CompactString::from("x").into()),
            TermNode::Number(Decimal::from(100)),
            TermNode::Placeholder(10.into()),
        ] {
            let config = config::standard();

            let encoded: Vec<u8> = bincode::encode_to_vec(t, config).unwrap();
            let (decoded, len): (TermNode, usize) =
                bincode::decode_from_slice(&encoded[..], config).unwrap();

            assert_eq!(t, &decoded);
            assert_eq!(len, encoded.len()); // read all bytes
        }
    }
    #[test]
    fn term_encode_test() {
        let term = term_with_params("a*x+b==0 => x==-b/a");
        let config = config::standard();

        let encoded: Vec<u8> = bincode::encode_to_vec(&term, config).unwrap();
        let (decoded, len): (Term, usize) =
            bincode::decode_from_slice(&encoded[..], config).unwrap();

        assert_eq!(term, decoded);
        assert_eq!(len, encoded.len());
    }
}
