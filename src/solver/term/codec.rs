use std::{
    collections::{HashMap, VecDeque},
    str::FromStr,
};

use bincode::{BorrowDecode, Decode, Encode};

use super::{SubtermId, Term, TermNode};
use crate::{CompactString, Decimal};

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
            TermNode::ArgList(p) => {
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
                TermNode::Number(Decimal::from_str(&s).map_err(|e| {
                    bincode::error::DecodeError::OtherString(format!(
                        "decimal parse error, str: '{s}': err: {e}"
                    ))
                })?)
            }
            5 => {
                let p: u64 = Decode::decode(decoder)?;
                TermNode::ArgList(p.into())
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
                TermNode::Number(Decimal::from_str(&s).map_err(|e| {
                    bincode::error::DecodeError::OtherString(format!(
                        "decimal parse error, str: '{s}': err: {e}"
                    ))
                })?)
            }
            5 => {
                let p: u64 = BorrowDecode::borrow_decode(decoder)?;
                TermNode::ArgList(p.into())
            }
            _ => unreachable!(),
        };
        Ok(term)
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for Term {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let parent_no: isize = Decode::decode(decoder)?;
        assert!(parent_no == -1, "root element must be first");
        let data: TermNode = Decode::decode(decoder)?;
        let mut term = Term::from(data);
        let mut id_map: HashMap<isize, SubtermId> =
            FromIterator::from_iter(vec![(0, term.as_subterm().id())]);

        for num in 1.. {
            let parent_no: isize = Decode::decode(decoder)?;
            if parent_no == -2 {
                break;
            }
            let parent_id = id_map.get(&parent_no).expect("parent must be first");

            let data: TermNode = Decode::decode(decoder)?;
            id_map.insert(
                num,
                term.get_mut(parent_id)
                    .unwrap()
                    .push_last_arg(Term::from(data))
                    .last_arg()
                    .unwrap()
                    .id(),
            );
        }
        Ok(term)
    }
}

impl<Context> Decode<Context> for Term {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let root_degree: usize = Decode::decode(decoder)?;
        let data: TermNode = Decode::decode(decoder)?;
        let mut root = Term::from(data);

        let mut queue = VecDeque::new();
        queue.push_back((SubtermId::default(), root_degree));

        while let Some((id, degree)) = queue.pop_front() {
            for _ in 0..degree {
                let node_degree: usize = Decode::decode(decoder)?;
                let data: TermNode = Decode::decode(decoder)?;
                let node = Term::from(data);

                let mut current = root.get_mut(&id).unwrap();
                current.push_last_arg(node);
                if node_degree > 0 {
                    let mut id = id.clone();
                    id.0.push(current.degree() - 1);
                    queue.push_back((id, node_degree));
                }
            }
        }
        let finish: isize = Decode::decode(decoder)?;
        assert_eq!(finish, -2);
        Ok(root)
    }
}

impl Encode for Term {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        for i in self.as_subterm().bfs() {
            Encode::encode(&i.size.degree, encoder)?;
            Encode::encode(i.data, encoder)?;
        }
        Encode::encode(&-2, encoder)?;
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
            TermNode::ArgList(10.into()),
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
