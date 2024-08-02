extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(FuncAttr)]
pub fn func_attr(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);

    let output = quote! {
        unsafe impl Sync for #ident {}
        unsafe impl Send for #ident {}

        impl fmt::Debug for #ident {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "")
            }
        }

        impl PartialEq for #ident {
            fn eq(&self, _: &Self) -> bool {
                true
            }
        }

        impl Eq for #ident {}
    };

    output.into()
}
