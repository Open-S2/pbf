use crate::FieldAttributes;
use darling::{FromField, FromVariant};
use proc_macro::TokenStream;
use quote::quote;
use syn::{DataEnum, DataStruct, Fields, GenericArgument, Ident, PathArguments, Type, TypePath};

pub fn derive_proto_read_struct(
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
                field_type_to_read_method(field_type, field_name, field_index, &attr, false)
                    .unwrap_or_else(|| {
                        panic!(
                            "Unsupported type in ProtoRead derive: {:#?}",
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
        panic!("ProtoRead can only be derived for structs with named fields");
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
            impl ProtoRead for #name {
                fn read(&mut self, tag: u64, pb: &mut Protobuf) {
                    match tag {
                        #(#write_statements)*
                        _ => panic!("unknown tag {}", tag),
                    }
                }
            }
        };
    };

    TokenStream::from(expanded)
}

/// Maps Rust types to the corresponding Protobuf read method.
fn field_type_to_read_method(
    field_type: &syn::Type,
    field_name: &Ident,
    field_index: u64,
    attr: &FieldAttributes,
    is_option: bool,
) -> Option<proc_macro2::TokenStream> {
    let field_index = attr.tag.unwrap_or(field_index);

    // Ensure the closure is called before inserting into quote!
    let wrap_option = |expr: proc_macro2::TokenStream| -> proc_macro2::TokenStream {
        if is_option {
            quote! { Some(#expr) }
        } else {
            expr
        }
    };

    match field_type {
        // Handling primitive types
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
            let read_method = if attr.signed {
                wrap_option(quote! { pb.read_s_varint() })
            } else if attr.fixed {
                wrap_option(quote! { pb.read_fixed() })
            } else {
                wrap_option(quote! { pb.read_varint() })
            };
            Some(quote! { #field_index => self.#field_name = #read_method, })
        }

        // Handling String fields
        Type::Path(TypePath { path, .. }) if path.is_ident("String") => {
            let read_string = wrap_option(quote! { pb.read_string() });
            Some(quote! { #field_index => self.#field_name = #read_string, })
        }

        // Handling Vec<T>
        Type::Path(TypePath { path, .. }) if path.segments.last().unwrap().ident == "Vec" => {
            if let PathArguments::AngleBracketed(ref args) = path.segments.last().unwrap().arguments
            {
                if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                    if let Type::Path(TypePath { path, .. }) = inner_type {
                        if path.is_ident("u8") {
                            let read_method = wrap_option(quote! { pb.read_bytes() });
                            return Some(
                                quote! { #field_index => self.#field_name = #read_method, },
                            );
                        }
                    }
                    let read_packed = if attr.signed {
                        wrap_option(quote! { pb.read_s_packed() })
                    } else {
                        wrap_option(quote! { pb.read_packed() })
                    };
                    return Some(quote! { #field_index => self.#field_name = #read_packed, });
                }
            }
            None
        }

        // Handling Option<T>
        Type::Path(TypePath { path, .. }) if path.segments.last().unwrap().ident == "Option" => {
            if let PathArguments::AngleBracketed(ref args) = path.segments.last().unwrap().arguments
            {
                if let Some(GenericArgument::Type(ref inner_type)) = args.args.first() {
                    return field_type_to_read_method(
                        inner_type,
                        field_name,
                        field_index,
                        attr,
                        true,
                    );
                }
            }
            None
        }

        // Handling nested messages
        Type::Path(TypePath { .. }) if attr.nested => {
            let read_method = wrap_option(quote! { nested_value });
            Some(quote! {
                #field_index => {
                    let mut nested_value = #field_type::default();
                    pb.read_message(&mut nested_value);
                    self.#field_name = #read_method;
                }
            })
        }

        // Handling Enums (assuming they are stored as integers)
        Type::Path(TypePath { .. }) => {
            let read_enum = wrap_option(quote! { pb.read_varint() });
            Some(quote! { #field_index => self.#field_name = #read_enum, })
        }

        // Other unsupported types
        _ => None,
    }
}

pub fn derive_proto_read_enum(data_enum: &DataEnum, name: &Ident, pbf_core: &Ident) -> TokenStream {
    let mut write_statements = Vec::new();
    let mut field_index: u64 = 0; // Default tag assignment

    for variant in &data_enum.variants {
        let variant_name = &variant.ident;
        let attr = FieldAttributes::from_variant(variant).unwrap();
        field_index = attr.tag.unwrap_or(field_index);
        if variant.fields.is_empty() {
            write_statements.push(quote! {
                #field_index => #name::#variant_name,
            });
        } else {
            for field in variant.fields.iter() {
                let field_type = &field.ty;
                // skip user defined "ignore"s
                if attr.ignore {
                    continue;
                }

                let write_method =
                    field_type_to_read_enum(field_type, name, variant_name, &attr, false)
                        .unwrap_or_else(|| {
                            panic!(
                                "Unsupported type in ProtoRead derive: {:#?}",
                                quote! { #field_type }
                            )
                        });
                write_statements.push(quote! {
                    #field_index => {
                        #write_method
                    }
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
            impl ProtoRead for #name {
                fn read(&mut self, tag: u64, pb: &mut Protobuf) {
                    *self = match tag {
                        #(#write_statements)*
                        _ => panic!("unknown tag {}", tag),
                    }
                }
            }
        };
    };

    TokenStream::from(expanded)
}

fn field_type_to_read_enum(
    field_type: &syn::Type,
    name: &Ident,
    variant_name: &Ident,
    attr: &FieldAttributes,
    is_option: bool,
) -> Option<proc_macro2::TokenStream> {
    // Ensure the closure is called before inserting into quote!
    let wrap_option = |expr: proc_macro2::TokenStream| -> proc_macro2::TokenStream {
        if is_option {
            quote! { Some(#expr) }
        } else {
            expr
        }
    };

    match field_type {
        // Handling primitive types
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
            let read_method = if attr.signed {
                wrap_option(quote! { pb.read_s_varint() })
            } else if attr.fixed {
                wrap_option(quote! { pb.read_fixed() })
            } else {
                wrap_option(quote! { pb.read_varint() })
            };

            Some(quote! { #name::#variant_name(#read_method) })
        }

        // Handling String fields
        Type::Path(TypePath { path, .. }) if path.is_ident("String") => {
            let read_method = wrap_option(quote! { pb.read_string() });
            Some(quote! { #name::#variant_name(#read_method) })
        }

        // Handling Vec<T>
        Type::Path(TypePath { path, .. }) if path.segments.last().unwrap().ident == "Vec" => {
            if let PathArguments::AngleBracketed(ref args) = path.segments.last().unwrap().arguments
            {
                if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                    if let Type::Path(TypePath { path, .. }) = inner_type {
                        if path.is_ident("u8") {
                            let read_method = wrap_option(quote! { pb.read_bytes() });
                            return Some(quote! { #name::#variant_name(#read_method) });
                        }
                    }
                    let read_packed = if attr.signed {
                        wrap_option(quote! { pb.read_s_packed() })
                    } else {
                        wrap_option(quote! { pb.read_packed() })
                    };
                    return Some(quote! { #name::#variant_name(#read_packed) });
                }
            }
            None
        }

        // Handling Option<T>
        Type::Path(TypePath { path, .. }) if path.segments.last().unwrap().ident == "Option" => {
            if let PathArguments::AngleBracketed(ref args) = path.segments.last().unwrap().arguments
            {
                if let Some(GenericArgument::Type(ref inner_type)) = args.args.first() {
                    return field_type_to_read_enum(inner_type, name, variant_name, attr, true);
                }
            }
            None
        }

        // Handling nested messages
        Type::Path(TypePath { .. }) if attr.nested => {
            let read_method = wrap_option(quote! { nested_value });
            Some(quote! {{
                let mut nested_value = #field_type::default();
                pb.read_message(&mut nested_value);
                #name::#variant_name(#read_method)
            }})
        }

        // Handling Enums (assuming they are stored as integers)
        Type::Path(TypePath { .. }) => {
            let read_method = wrap_option(quote! { pb.read_varint() });
            Some(quote! { #name::#variant_name(#read_method) })
        }

        // Other unsupported types
        _ => None,
    }
}
