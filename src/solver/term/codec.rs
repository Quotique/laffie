use bincode::{BorrowDecode, Decode, Encode};
use trees::tr;

use crate::symbol::{codec::encode_node, Symbol};

use super::{SymbolTree, Term};

impl<'de> BorrowDecode<'de> for Term {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let mut tree_vec: Vec<(isize, SymbolTree)> = vec![];
        loop {
            let parent_no: isize = BorrowDecode::borrow_decode(decoder)?;
            if parent_no == -2 {
                break;
            }

            let data: Symbol = BorrowDecode::borrow_decode(decoder)?;
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

impl Decode for Term {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let mut tree_vec: Vec<(isize, SymbolTree)> = vec![];
        loop {
            let parent_no: isize = Decode::decode(decoder)?;
            if parent_no == -2 {
                break;
            }

            let data: Symbol = Decode::decode(decoder)?;
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
        encode_node(-1, &mut last_no, self.root(), encoder)?;
        Encode::encode(&-2, encoder)?;
        assert!(self.binds.is_empty(), "binds encode is not supported");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bincode::config;

    use crate::term::{term_with_params, Term};

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
