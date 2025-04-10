use std::str::FromStr;

use bincode::{BorrowDecode, Decode, Encode};

use crate::{symbol::Symbol, CompactString, Decimal};

use super::SymbolNode;

impl Encode for Symbol {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        match self {
            Symbol::FuncSymbol(s) => {
                Encode::encode(&1i8, encoder)?;
                Encode::encode(&s.name.as_str(), encoder)?;
            }
            Symbol::Param(p) => {
                Encode::encode(&2i8, encoder)?;
                Encode::encode(&p.as_ref().as_str(), encoder)?;
            }
            Symbol::Variable(v) => {
                Encode::encode(&3i8, encoder)?;
                Encode::encode(&v.as_ref().as_str(), encoder)?;
            }
            Symbol::Number(d) => {
                Encode::encode(&4i8, encoder)?;
                Encode::encode(&d.to_string().as_str(), encoder)?;
            }
            Symbol::Placeholder(p) => {
                Encode::encode(&5i8, encoder)?;
                Encode::encode(&u64::from(*p), encoder)?;
            }
        }
        Ok(())
    }
}

impl<Context> Decode<Context> for Symbol {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let index: i8 = Decode::decode(decoder)?;
        let term = match index {
            1 => {
                let s: String = Decode::decode(decoder)?;
                Symbol::with_func_symbol_opt(&s).ok_or_else(|| {
                    bincode::error::DecodeError::OtherString(format!("bad symbol {s}"))
                })?
            }
            2 => {
                let s: String = Decode::decode(decoder)?;
                Symbol::Param(CompactString::from(s).into())
            }
            3 => {
                let s: String = Decode::decode(decoder)?;
                Symbol::Variable(CompactString::from(s).into())
            }
            4 => {
                let s: String = Decode::decode(decoder)?;
                // TODO: remove unwrap
                Symbol::Number(Decimal::from_str(&s).unwrap())
            }
            5 => {
                let p: u64 = Decode::decode(decoder)?;
                Symbol::Placeholder(p.into())
            }
            _ => unreachable!(),
        };
        Ok(term)
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for Symbol {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let index: i8 = BorrowDecode::borrow_decode(decoder)?;
        let term = match index {
            1 => {
                let s: String = BorrowDecode::borrow_decode(decoder)?;
                Symbol::with_func_symbol_opt(&s).ok_or_else(|| {
                    bincode::error::DecodeError::OtherString(format!("bad symbol {s}"))
                })?
            }
            2 => {
                let s: String = BorrowDecode::borrow_decode(decoder)?;
                Symbol::Param(CompactString::from(s).into())
            }
            3 => {
                let s: String = BorrowDecode::borrow_decode(decoder)?;
                Symbol::Variable(CompactString::from(s).into())
            }
            4 => {
                let s: String = BorrowDecode::borrow_decode(decoder)?;
                // TODO: remove unwrap
                Symbol::Number(Decimal::from_str(&s).unwrap())
            }
            5 => {
                let p: u64 = BorrowDecode::borrow_decode(decoder)?;
                Symbol::Placeholder(p.into())
            }
            _ => unreachable!(),
        };
        Ok(term)
    }
}

pub fn encode_node<E: bincode::enc::Encoder>(
    parent_no: isize,
    last_no: &mut isize,
    node: &SymbolNode,
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

    use crate::{symbol::Symbol, CompactString, Decimal};

    #[test]
    fn symbol_codec_test() {
        for t in &[
            Symbol::with_func_symbol("+"),
            Symbol::Param(CompactString::from("a").into()),
            Symbol::Variable(CompactString::from("x").into()),
            Symbol::Number(Decimal::from(100)),
            Symbol::Placeholder(10.into()),
        ] {
            let config = config::standard();

            let encoded: Vec<u8> = bincode::encode_to_vec(t, config).unwrap();
            let (decoded, len): (Symbol, usize) =
                bincode::decode_from_slice(&encoded[..], config).unwrap();

            assert_eq!(t, &decoded);
            assert_eq!(len, encoded.len()); // read all bytes
        }
    }
}
