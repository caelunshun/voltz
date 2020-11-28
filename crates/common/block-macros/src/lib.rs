use std::ops::{Deref, RangeInclusive};

use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use proc_macro_error::{abort, abort_call_site, emit_error, proc_macro_error};
use quote::quote;
use syn::{
    spanned::Spanned, DeriveInput, Expr, ExprParen, ExprRange, Field, Ident, ItemStruct, Lit, Path,
    RangeLimits, Type,
};

#[derive(FromDeriveInput)]
#[darling(attributes(block))]
struct Descriptor {
    slug: String,
    display_name: String,
}

/// Used to create a struct representing a block plus
/// several helper structs.
///
/// * Implements the `Block` trait for the given struct.
/// * Generates getters and setters for each field which
/// check that values are valid.
/// * Generates a descriptor struct which can be converted
/// to an instance of this struct, for easy builder-like construction.
///
/// # Fields
/// Fields of the given struct are block properties. Internally, block
/// properties are stored as a single `u32` for an entire block. As a result,
/// the number of possible combinations of block properties for this struct
/// must not exceed `u32::MAX`.
///
/// For numeric properties (e.g. `u32`), you need to specify a range of legal
/// values. Otherwise, the total number of values will exceed the above limit.
/// For enum or bool properties, this is not necessary.
///
/// Block properties must implement the `BlockProperty` trait.
#[proc_macro_derive(Block, attributes(range, block))]
#[proc_macro_error]
pub fn block(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input1 = input.clone();
    let derive_input = syn::parse_macro_input!(input1 as DeriveInput);
    let descriptor = Descriptor::from_derive_input(&derive_input)
        .unwrap_or_else(|_| abort_call_site!("failed to parse block descriptor tag"));

    let input = syn::parse_macro_input!(input as ItemStruct);
    let fields = validate(&input);

    let properties = determine_properties(&fields);

    let block_impl = generate_block_impl(&input, &descriptor, &properties);

    let result = quote! {
        #block_impl
    };
    result.into()
}

fn validate(input: &ItemStruct) -> Vec<Field> {
    if !input.generics.params.is_empty() {
        emit_error! {
            input.generics.params.first().unwrap().span(),
            "block struct cannot have generic parameters"
        }
    }

    match &input.fields {
        syn::Fields::Named(fields) => fields.named.iter().cloned().collect(),
        syn::Fields::Unnamed(unnamed) => abort! {
            unnamed.span(),
            "tuple structs are not supported"
        },
        syn::Fields::Unit => Vec::new(),
    }
}

type Properties = Vec<Property>;

enum Property {
    Integer {
        range: RangeInclusive<i64>,
        ident: Ident,
    },
    Other {
        typ: Type,
        ident: Ident,
    },
}

fn determine_properties(fields: &[Field]) -> Properties {
    let mut properties = Properties::new();

    for field in fields {
        let typ = &field.ty;
        let property = match typ {
            Type::Path(path) => convert_property(field, &path.path),
            _ => abort! {
                typ.span(),
                "unsupported block property type"
            },
        };
        properties.push(property);
    }

    properties
}

fn convert_property(field: &Field, path: &Path) -> Property {
    if path.is_ident("i32") || path.is_ident("u32") {
        let range = parse_integer_range_attr(field);
        Property::Integer {
            range,
            ident: field.ident.as_ref().unwrap().clone(),
        }
    } else {
        Property::Other {
            typ: field.ty.clone(),
            ident: field.ident.as_ref().unwrap().clone(),
        }
    }
}

fn parse_integer_range_attr(field: &Field) -> RangeInclusive<i64> {
    let range_attr = match field.attrs.iter().find(|attr| attr.path.is_ident("range")) {
        Some(attr) => attr,
        None => {
            abort! { field.span(), "integer property must specify a #[range(a..=b)] attribute"}
        }
    };
    let tokens = &range_attr.tokens;
    let range: syn::ExprRange =
        match syn::parse::<ExprParen>(proc_macro::TokenStream::from(tokens.clone())) {
            Ok(r) => match r.expr.deref() {
                Expr::Range(range) => range.clone(),
                _ => abort! { r.span(), "expected range" },
            },
            Err(e) => abort! {
                tokens.span(),
                "failed to parse integer range: {}", e
            },
        };
    parse_range(range)
}

fn parse_range(range: ExprRange) -> RangeInclusive<i64> {
    let from: i64 = parse_range_lit(&range.from);
    let to: i64 = parse_range_lit(&range.to);

    match range.limits {
        RangeLimits::HalfOpen(_) => from..=to - 1,
        RangeLimits::Closed(_) => from..=to,
    }
}

fn parse_range_lit(expr: &Option<Box<Expr>>) -> i64 {
    match expr {
        Some(from) => match from.deref() {
            Expr::Lit(lit) => match &lit.lit {
                Lit::Int(int) => int.base10_parse().expect("integer failed to parse"),
                _ => abort! { lit.lit.span(), "range literal must be integer" },
            },
            _ => abort! {
                from.span(), "range must contain integer literals"
            },
        },
        None => abort! { expr.span(), "range must contain start/end" },
    }
}

fn generate_block_impl(
    item: &ItemStruct,
    descriptor: &Descriptor,
    properties: &Properties,
) -> TokenStream {
    let ident = &item.ident;
    let packer = quote::format_ident!("ImplBlockFor_{}_PropertyPacker", ident);

    let num_properties = properties.len();

    let num_possible_values = generate_num_possible_values(properties);
    let map_prop_to_int: Vec<TokenStream> = generate_map_prop_to_int(properties);
    let map_int_to_prop: Vec<TokenStream> = generate_map_int_to_prop(properties);

    let Descriptor { slug, display_name } = descriptor;

    quote! {
        #[allow(non_upper_case_globals)]
        static #packer: crate::block::PropertyPacker<#num_properties> = crate::block::PropertyPacker::new([#(#num_possible_values),*]);

        impl crate::block::Block for #ident {
            fn state_id(&self) -> u32 {
                #packer.pack([#(
                    #map_prop_to_int
                ),*])
            }

            fn from_state_id(state: u32) -> Option<Self> {
                let unpacked = #packer.unpack(state);

                Some(Self {
                    #(
                        #map_int_to_prop,
                    )*
                })
            }

            fn descriptor() -> crate::block::BlockDescriptor {
                crate::block::BlockDescriptor::new(#slug, #display_name)
            }
        }
    }
}

fn generate_num_possible_values(properties: &Properties) -> Vec<TokenStream> {
    properties
        .iter()
        .map(|property| match property {
            Property::Integer { range, .. } => {
                let x = (range.end() - range.start() + 1) as u32;
                quote! { #x }
            }
            Property::Other { typ, .. } => {
                quote! {
                    { <#typ as crate::block::BlockProperty>::NUM_POSSIBLE_VALUES }
                }
            }
        })
        .collect()
}

fn generate_map_prop_to_int(properties: &Properties) -> Vec<TokenStream> {
    properties
        .iter()
        .map(|property| match property {
            Property::Integer { range, ident, .. } => {
                let start = *range.start();
                quote! { self.#ident - #start as u32 }
            }
            Property::Other { typ, ident } => {
                quote! { <#typ as crate::block::BlockProperty>::to_int(self.#ident) }
            }
        })
        .collect()
}

fn generate_map_int_to_prop(properties: &Properties) -> Vec<TokenStream> {
    properties
        .iter()
        .enumerate()
        .map(|(i, property)| match property {
            Property::Integer { range, ident, .. } => {
                let start = *range.start();
                quote! {
                    #ident: unpacked[#i] + #start as u32
                }
            }
            Property::Other { typ, ident } => {
                quote! {
                    #ident: <#typ as crate::block::BlockProperty>::from_int(unpacked[#i])?
                }
            }
        })
        .collect()
}
