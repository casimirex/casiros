//! CASIROS Narrative Engine — procedural macros.
//!
//! Provides compile-time narrative generation for financial reports and
//! "CFO Memo" outputs.
//!
//! ## Layer
//!
//! Infrastructure Layer — used by the API and future presentation layers.

use proc_macro::TokenStream;
use quote::quote;

/// Stub narrative macro for the MVP.
///
/// Full implementation will accept key-value metric pairs and expand to a
/// formatted `String`.
#[proc_macro]
pub fn generate_narrative(_input: TokenStream) -> TokenStream {
    let output = quote! {
        "CASIROS narrative placeholder".to_string()
    };
    output.into()
}
