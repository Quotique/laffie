use std::{
    collections::{HashMap, VecDeque},
    str::FromStr,
};

use bincode::{BorrowDecode, Decode, Encode, error::DecodeError};

use super::{Atom, TermBuf, TermPath};
use crate::{CompactString, Decimal, term::try_sym};

impl Encode for Atom {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        match self {
            Atom::Symbol(s) => {
                Encode::encode(&1i8, encoder)?;
                Encode::encode(&s.as_str(), encoder)?;
            }
            Atom::Param(p) => {
                Encode::encode(&2i8, encoder)?;
                Encode::encode(&p.as_ref().as_str(), encoder)?;
            }
            Atom::Variable(v) => {
                Encode::encode(&3i8, encoder)?;
                Encode::encode(&v.as_ref().as_str(), encoder)?;
            }
            Atom::Number(d) => {
                Encode::encode(&4i8, encoder)?;
                Encode::encode(&d.to_string().as_str(), encoder)?;
            }
            Atom::ArgList(p) => {
                Encode::encode(&5i8, encoder)?;
                Encode::encode(&u64::from(*p), encoder)?;
            }
        }
        Ok(())
    }
}

impl<Context> Decode<Context> for Atom {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let index: i8 = Decode::decode(decoder)?;
        let term = match index {
            1 => {
                let s: String = Decode::decode(decoder)?;
                Atom::from(
                    try_sym(&s)
                        .ok_or_else(|| DecodeError::OtherString(format!("bad symbol {s}")))?,
                )
            }
            2 => {
                let s: String = Decode::decode(decoder)?;
                Atom::Param(CompactString::from(s).into())
            }
            3 => {
                let s: String = Decode::decode(decoder)?;
                Atom::Variable(CompactString::from(s).into())
            }
            4 => {
                let s: String = Decode::decode(decoder)?;
                Atom::Number(Decimal::from_str(&s).map_err(|e| {
                    DecodeError::OtherString(format!("decimal parse error, str: '{s}': err: {e}"))
                })?)
            }
            5 => {
                let p: u64 = Decode::decode(decoder)?;
                Atom::ArgList(p.into())
            }
            _ => {
                return Err(DecodeError::OtherString(format!(
                    "unknown Atom tag: {index}"
                )));
            }
        };
        Ok(term)
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for Atom {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let index: i8 = BorrowDecode::borrow_decode(decoder)?;
        let term = match index {
            1 => {
                let s: String = BorrowDecode::borrow_decode(decoder)?;
                Atom::from(
                    try_sym(&s)
                        .ok_or_else(|| DecodeError::OtherString(format!("bad symbol {s}")))?,
                )
            }
            2 => {
                let s: String = BorrowDecode::borrow_decode(decoder)?;
                Atom::Param(CompactString::from(s).into())
            }
            3 => {
                let s: String = BorrowDecode::borrow_decode(decoder)?;
                Atom::Variable(CompactString::from(s).into())
            }
            4 => {
                let s: String = BorrowDecode::borrow_decode(decoder)?;
                Atom::Number(Decimal::from_str(&s).map_err(|e| {
                    DecodeError::OtherString(format!("decimal parse error, str: '{s}': err: {e}"))
                })?)
            }
            5 => {
                let p: u64 = BorrowDecode::borrow_decode(decoder)?;
                Atom::ArgList(p.into())
            }
            _ => {
                return Err(DecodeError::OtherString(format!(
                    "unknown Atom tag: {index}"
                )));
            }
        };
        Ok(term)
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for TermBuf {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let parent_no: isize = Decode::decode(decoder)?;
        if parent_no != -1 {
            return Err(DecodeError::OtherString(
                "root element must be first".into(),
            ));
        }
        let data: Atom = Decode::decode(decoder)?;
        let mut term = TermBuf::from(data);
        let mut id_map: HashMap<isize, TermPath> =
            FromIterator::from_iter(vec![(0, term.term().id())]);

        for num in 1.. {
            let parent_no: isize = Decode::decode(decoder)?;
            if parent_no == -2 {
                break;
            }
            let parent_id = id_map.get(&parent_no).expect("parent must be first");

            let data: Atom = Decode::decode(decoder)?;
            id_map.insert(
                num,
                term.get_mut(parent_id)
                    .unwrap()
                    .push_last_arg(TermBuf::from(data))
                    .last_arg()
                    .unwrap()
                    .id(),
            );
        }
        Ok(term)
    }
}

impl<Context> Decode<Context> for TermBuf {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let root_degree: usize = Decode::decode(decoder)?;
        let data: Atom = Decode::decode(decoder)?;
        let mut root = TermBuf::from(data);

        let mut queue = VecDeque::new();
        queue.push_back((TermPath::default(), root_degree));

        while let Some((id, degree)) = queue.pop_front() {
            for _ in 0..degree {
                let node_degree: usize = Decode::decode(decoder)?;
                let data: Atom = Decode::decode(decoder)?;
                let node = TermBuf::from(data);

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
        if finish != -2 {
            return Err(DecodeError::OtherString(format!(
                "expected terminator -2, got {finish}"
            )));
        }
        Ok(root)
    }
}

impl Encode for TermBuf {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        for i in self.term().bfs() {
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
        CompactString, Decimal,
        term::{Atom, TermBuf, sym, term_with_params},
    };

    #[test]
    fn symbol_codec_test() {
        for t in &[
            Atom::from(sym("+")),
            Atom::Param(CompactString::from("a").into()),
            Atom::Variable(CompactString::from("x").into()),
            Atom::Number(Decimal::from(100)),
            Atom::ArgList(10.into()),
        ] {
            let config = config::standard();

            let encoded: Vec<u8> = bincode::encode_to_vec(t, config).unwrap();
            let (decoded, len): (Atom, usize) =
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
        let (decoded, len): (TermBuf, usize) =
            bincode::decode_from_slice(&encoded[..], config).unwrap();

        assert_eq!(term, decoded);
        assert_eq!(len, encoded.len());
    }
}
