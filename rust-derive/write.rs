use crate::FieldAttributes;
use darling::{FromField, FromVariant};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{DataEnum, DataStruct, Fields, GenericArgument, Ident, PathArguments, Type, TypePath};

pub fn derive_proto_write_struct(
    data_struct: &DataStruct,
    name: &Ident,
    pbf_core: &Ident,
) -> TokenStream {
    let mut write_statements = Vec::new();
    let mut field_index: u64 = 0; // Default tag assignment

    if let Fields::Named(fields) = &data_struct.fields {
        for field in &fields.named {
            let field_name = field.ident.as_ref().unwrap();
            let field_type = &field.ty;
            let attr = FieldAttributes::from_field(field).unwrap();
            // skip user defined "ignore"s
            if attr.ignore {
                continue;
            }

            let write_method =
                field_type_to_write_method(field_type, field_name, field_index, &attr, false)
                    .unwrap_or_else(|| {
                        panic!(
                            "Unsupported type in ProtoWrite derive: {:#?}",
                            quote! { #field_type }
                        )
                    });

            write_statements.push(write_method);
            // increment field_index only if the user did not define an index for the field
            if let Some(index) = attr.tag {
                field_index = index + 1;
            } else {
                field_index += 1;
            }
        }
    } else {
        panic!("ProtoWrite can only be derived for structs with named fields");
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
            impl ProtoWrite for #name {
                fn write(&self, pbf: &mut Protobuf) {
                    #(#write_statements)*
                }
            }
        };
    };

    TokenStream::from(expanded)
}

pub fn derive_proto_write_enum(
    data_enum: &DataEnum,
    name: &Ident,
    pbf_core: &Ident,
) -> TokenStream {
    let mut write_statements = Vec::new();
    let mut field_index: u64 = 0; // Default tag assignment

    for variant in &data_enum.variants {
        let variant_name = &variant.ident;
        let attr = FieldAttributes::from_variant(variant).unwrap();
        if variant.fields.is_empty() {
            write_statements.push(quote! {
                #name::#variant_name => pbf.write_field(#field_index, Type::None),
            });
        } else {
            for (idx, field) in variant.fields.iter().enumerate() {
                let field_name = format_ident!("field{}", idx); // Generate a field name
                let field_type = &field.ty;
                // skip user defined "ignore"s
                if attr.ignore {
                    continue;
                }

                let write_method =
                    field_type_to_write_method(field_type, &field_name, field_index, &attr, true)
                        .unwrap_or_else(|| {
                            panic!(
                                "Unsupported type in ProtoWrite derive: {:#?}",
                                quote! { #field_type }
                            )
                        });

                write_statements.push(quote! {
                    #name::#variant_name(#field_name) => {
                        #write_method
                    },
                });
            }
            // increment field_index only if the user did not define an index for the field
            if let Some(index) = attr.tag {
                field_index = index + 1;
            } else {
                field_index += 1;
            }
        }
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
            impl ProtoWrite for #name {
                fn write(&self, pbf: &mut Protobuf) {
                    match self {
                        #(#write_statements)*
                    }
                }
            }
        };
    };

    TokenStream::from(expanded)
}

/// Maps Rust types to the corresponding Protobuf write method.
fn field_type_to_write_method(
    field_type: &syn::Type,
    field_name: &Ident,
    field_index: u64,
    attr: &FieldAttributes,
    is_option: bool,
) -> Option<proc_macro2::TokenStream> {
    let name: proc_macro2::TokenStream = if is_option {
        quote! { #field_name }
    } else {
        quote! { self.#field_name }
    };
    let name_st = if is_option {
        quote! { *#name }
    } else {
        quote! { #name }
    };
    let field_index = attr.tag.unwrap_or(field_index);
    match field_type {
        // Handling leftover primitive types
        Type::Path(TypePath { path, .. })
            if path.is_ident("u8")
                || path.is_ident("i8")
                || path.is_ident("u16")
                || path.is_ident("i16")
                || path.is_ident("u32")
                || path.is_ident("i32")
                || path.is_ident("f32")
                || path.is_ident("u64")
                || path.is_ident("i64")
                || path.is_ident("f64")
                || path.is_ident("usize")
                || path.is_ident("isize")
                || path.is_ident("bool") =>
        {
            if attr.signed {
                Some(quote! { pbf.write_s_varint_field(#field_index, #name_st); })
            } else if attr.fixed {
                Some(quote! { pbf.write_fixed_field(#field_index, #name_st); })
            } else {
                Some(quote! { pbf.write_varint_field(#field_index, #name_st); })
            }
        }

        // Handling String (could be treated as a write_string_field)
        Type::Path(TypePath { path, .. }) if path.is_ident("String") => {
            Some(quote! { pbf.write_string_field(#field_index, &#name); })
        }

        // Handling Vec<T> (bytes fields)
        Type::Path(TypePath { path, .. }) if path.segments.last().unwrap().ident == "Vec" => {
            if let PathArguments::AngleBracketed(ref args) = path.segments.last().unwrap().arguments
            {
                if let Some(GenericArgument::Type(Type::Path(TypePath { path, .. }))) =
                    args.args.first()
                {
                    if path.segments.last().unwrap().ident == "u8" {
                        // If the type inside Vec is u8, use write_bytes_field
                        return Some(quote! { pbf.write_bytes_field(#field_index, &#name_st); });
                    } else {
                        // Otherwise, use packed
                        if attr.signed {
                            return Some(
                                quote! { pbf.write_packed_s_varint(#field_index, &#name_st); },
                            );
                        } else {
                            return Some(
                                quote! { pbf.write_packed_varint(#field_index, &#name_st); },
                            );
                        }
                    }
                }
            }
            None
        }

        // Handling Option<T>
        Type::Path(TypePath { path, .. }) if path.segments.last().unwrap().ident == "Option" => {
            if let PathArguments::AngleBracketed(ref args) = path.segments.last().unwrap().arguments
            {
                if let Some(GenericArgument::Type(ref inner_type)) = args.args.first() {
                    if let Some(internal_field) =
                        field_type_to_write_method(inner_type, field_name, field_index, attr, true)
                    {
                        return Some(quote! {
                            if let Some(#field_name) = &#name {
                                #internal_field
                            }
                        });
                    }
                }
            }
            None
        }

        // Detecting Structs
        Type::Path(TypePath { .. }) if attr.nested => {
            Some(quote! { pbf.write_message(#field_index, &#name_st); })
        }

        // Assume last case is an enum
        Type::Path(TypePath { .. }) => {
            Some(quote! { pbf.write_varint_field(#field_index, #name_st); })
        }

        // Other types (e.g., arrays or references can be extended here)
        _ => None, // You could return Option::None for unsupported types or handle them
    }
}
