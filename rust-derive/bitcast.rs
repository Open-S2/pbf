use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident};

pub fn expand_bitcast(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let crate_name = match crate_name("pbf") {
        Ok(FoundCrate::Itself) => "pbf".to_string(),
        Ok(FoundCrate::Name(name)) => name,
        Err(_) => "pbf_core".to_string(), // Fallback if resolution fails (happens for testing)
    };
    let pbf_core = Ident::new(&crate_name, Span::call_site());

    // Ensure we are deriving for an enum
    let Data::Enum(enum_data) = &input.data else {
        panic!("BitCast can only be derived for enums");
    };

    // Extract variant names and discriminants
    let mut from_u64_arms = Vec::new();
    let mut to_u64_arms = Vec::new();

    for variant in &enum_data.variants {
        let variant_name = &variant.ident;

        // Ensure the variant has no fields (i.e., unit-like)
        if !matches!(variant.fields, Fields::Unit) {
            panic!("BitCast can only be derived for unit-like enums");
        }

        // Extract discriminant value
        let Some((_, expr)) = &variant.discriminant else {
            panic!("BitCast requires explicit discriminants on all variants");
        };

        from_u64_arms.push(quote! { #expr => Self::#variant_name });
        to_u64_arms.push(quote! { Self::#variant_name => #expr });
    }

    // Generate the trait implementation
    let expanded = quote! {
        #[doc(hidden)]
        #[allow(
            non_upper_case_globals,
            unused_attributes,
            unused_qualifications,
            clippy::absolute_paths,
        )]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate #pbf_core as _pbf_core;
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate alloc;

            use _pbf_core::*;

            #[automatically_derived]
            impl BitCast for #name {
                fn from_u64(val: u64) -> Self {
                    match val {
                        #(#from_u64_arms,)*
                        _ => panic!("Invalid enum value: {}", val),
                    }
                }

                fn to_u64(&self) -> u64 {
                    match self {
                        #(#to_u64_arms,)*
                    }
                }
            }
        };
    };

    TokenStream::from(expanded)
}
