#![forbid(unsafe_code)]

use darling::{self, FromField, FromVariant};
use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use syn::{Data, DeriveInput, Ident, parse_macro_input};

mod bitcast;
mod read;
mod write;

use bitcast::expand_bitcast;
use read::{derive_proto_read_enum, derive_proto_read_struct};
use write::{derive_proto_write_enum, derive_proto_write_struct};

#[derive(Debug, FromField, FromVariant)]
#[darling(attributes(pbf))]
struct FieldAttributes {
    tag: Option<u64>,
    #[darling(default)]
    signed: bool,
    #[darling(default)]
    fixed: bool,
    #[darling(default)]
    nested: bool,
    #[darling(default)]
    ignore: bool,
}

/// Derive the `BitCast` trait for an enum.
#[proc_macro_derive(BitCast)]
pub fn derive_bit_cast(input: TokenStream) -> TokenStream {
    expand_bitcast(input)
}

#[proc_macro_derive(ProtoWrite, attributes(pbf))]
pub fn derive_proto_write(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let crate_name = match crate_name("pbf") {
        Ok(FoundCrate::Itself) => "pbf".to_string(),
        Ok(FoundCrate::Name(name)) => name,
        Err(_) => "pbf_core".to_string(), // Fallback if resolution fails (happens for testing)
    };
    let pbf_core = Ident::new(&crate_name, Span::call_site());

    match &input.data {
        Data::Struct(data_struct) => derive_proto_write_struct(data_struct, name, &pbf_core),
        Data::Enum(data_enum) => derive_proto_write_enum(data_enum, name, &pbf_core),
        _ => panic!("ProtoWrite can only be derived for structs and enums"),
    }
}

#[proc_macro_derive(ProtoRead, attributes(pbf))]
pub fn derive_proto_read(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let crate_name = match crate_name("pbf") {
        Ok(FoundCrate::Itself) => "pbf".to_string(),
        Ok(FoundCrate::Name(name)) => name,
        Err(_) => "pbf_core".to_string(), // Fallback if resolution fails (happens for testing)
    };
    let pbf_core = Ident::new(&crate_name, Span::call_site());

    match &input.data {
        Data::Struct(data_struct) => derive_proto_read_struct(data_struct, name, &pbf_core),
        Data::Enum(data_enum) => derive_proto_read_enum(data_enum, name, &pbf_core),
        _ => panic!("ProtoRead can only be derived for structs and enums"),
    }
}
