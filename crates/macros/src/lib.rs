//! CASIROS Narrative Engine — procedural macros.
//!
//! Provides compile-time narrative generation for financial reports and
//! "CFO Memo" outputs. The primary entry point is the [`Narrative`]
//! derive macro, which generates a human-readable sentence from a struct's
//! fields.
//!
//! ## Layer
//!
//! Infrastructure Layer — used by the API and future presentation layers.
//!
//! ## Supported attributes
//!
//! - `#[narrative(skip)]` on a field — omit the field from the narrative.
//! - `#[narrative(name = "...")]` on a field — use a custom display name.
//! - `#[narrative(prefix = "...")]` on the struct — prepend a custom prefix to
//!   the narrative. Defaults to the struct name.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(warnings)]
#![allow(clippy::needless_return)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Attribute, Data, DataStruct, DeriveInput, Field, Lit, parse_macro_input};

/// Derive macro generating a `casiros_core::narrative::Narrative` implementation.
///
/// The generated `narrative()` method returns a sentence such as:
///
/// ```text
/// Wacc: equity_value = 600, debt_value = 400, cost_of_equity = 0.12, ...
/// ```
///
/// # Examples
///
/// ```ignore
/// use casiros_core::narrative::Narrative;
/// use casiros_macros::Narrative;
/// use rust_decimal_macros::dec;
///
/// #[derive(Narrative)]
/// #[narrative(prefix = "Capital structure")]
/// struct CapitalStructure {
///     equity: rust_decimal::Decimal,
///     #[narrative(name = "total debt")]
///     debt: rust_decimal::Decimal,
/// }
///
/// let cs = CapitalStructure { equity: dec!(600.0), debt: dec!(400.0) };
/// assert!(cs.narrative().contains("equity = 600"));
/// assert!(cs.narrative().contains("total debt = 400"));
/// ```
#[proc_macro_derive(Narrative, attributes(narrative))]
pub fn derive_narrative(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    let Data::Struct(DataStruct {
        fields: syn::Fields::Named(named),
        ..
    }) = input.data
    else {
        return syn::Error::new(
            Span::call_site(),
            "Narrative can only be derived for structs with named fields",
        )
        .to_compile_error()
        .into();
    };

    let default_prefix = struct_name.to_string();
    let prefix = match struct_prefix(&input.attrs, &default_prefix) {
        Ok(value) => value,
        Err(err) => return err.to_compile_error().into(),
    };

    let mut fragments = Vec::new();
    for field in &named.named {
        let fragment = match field_fragment(field) {
            Ok(Some(value)) => value,
            Ok(None) => continue,
            Err(err) => return err.to_compile_error().into(),
        };
        fragments.push(fragment);
    }

    let expanded = quote! {
        impl casiros_core::narrative::Narrative for #struct_name {
            fn narrative(&self) -> String {
                let mut parts: Vec<String> = Vec::new();
                #(#fragments)*
                let joined = parts.join(", ");
                if joined.is_empty() {
                    return format!("{}: (empty)", #prefix);
                }
                return format!("{}: {}", #prefix, joined);
            }
        }
    };

    return expanded.into();
}

fn struct_prefix(attrs: &[Attribute], default: &str) -> Result<String, syn::Error> {
    for attr in attrs {
        if !attr.path().is_ident("narrative") {
            continue;
        }

        let mut prefix: Option<String> = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("prefix") {
                let value = meta.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(lit_str) = lit {
                    prefix = Some(lit_str.value());
                    return Ok(());
                }
                return Err(meta.error("prefix must be a string literal"));
            }
            Ok(())
        })?;

        if let Some(value) = prefix {
            return Ok(value);
        }
    }
    return Ok(default.to_string());
}

fn field_fragment(field: &Field) -> Result<Option<proc_macro2::TokenStream>, syn::Error> {
    let ident = field
        .ident
        .as_ref()
        .expect("named fields always have an identifier");
    let field_name = ident.to_string();

    let mut display_name = field_name.clone();
    let mut skip = false;

    for attr in &field.attrs {
        if !attr.path().is_ident("narrative") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                skip = true;
                return Ok(());
            }
            if meta.path.is_ident("name") {
                let value = meta.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(lit_str) = lit {
                    display_name = lit_str.value();
                    return Ok(());
                }
                return Err(meta.error("name must be a string literal"));
            }
            if meta.path.is_ident("prefix") {
                // prefix is only valid on the struct, not on fields.
                return Err(meta.error("prefix is only allowed on the struct"));
            }
            Ok(())
        })?;
    }

    if skip {
        return Ok(None);
    }

    let fragment = quote! {
        parts.push(format!("{} = {}", #display_name, self.#ident));
    };
    return Ok(Some(fragment));
}
