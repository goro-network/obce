use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse2,
    Error,
    ItemTrait,
    Lit,
    Meta,
    NestedMeta,
    TraitItem,
    TraitItemMethod, FnArg, ReturnType, Attribute, parse_quote,
};

use crate::{
    format_err_spanned,
    types::AttributeArgs,
    utils::{into_u16, into_u32},
};

struct TraitAttrs {
    id: u16,
    // testing: bool,
}

impl TraitAttrs {
    fn new(trait_item: &ItemTrait, attrs: AttributeArgs) -> Result<Self, Error> {
        let id = find_id(&attrs)?.unwrap_or_else(|| into_u16(&trait_item.ident));

        // let testing = attrs.iter().any(|arg| {
        //     matches!(
        //         arg,
        //         NestedMeta::Meta(Meta::Path(value))
        //         if value.is_ident("testing")
        //     )
        // });

        Ok(Self { id })
    }
}

struct Method {
    id: u16,
    hash: u32,
    input_tokens: TokenStream,
    output_tokens: TokenStream,
}

impl Method {
    fn new(method_item: &mut TraitItemMethod) -> Result<Self, Error> {
        if let Some(default) = &method_item.default {
            return Err(format_err_spanned!(
                default,
                "default implementation is not supported in chain extensions"
            ))
        }

        let (obce_attrs, other_attrs) = method_item
            .attrs
            .iter()
            .cloned()
            .partition::<Vec<_>, _>(|attr| attr.path.is_ident("obce"));

        method_item.attrs = other_attrs;

        // FIXME: Handle multiple obce attrs gracefully
        let id = obce_attrs
            .first()
            .map(Attribute::parse_args)
            .transpose()?
            .as_ref()
            .and_then(|attrs| find_id(attrs).transpose())
            .unwrap_or_else(|| Ok(into_u16(&method_item.sig.ident)))?;

        let hash = into_u32(&method_item.sig.ident);

        let input_tys = method_item.sig
            .inputs
            .iter()
            .filter_map(|input| if let FnArg::Typed(pat) = input {
                Some(&*pat.ty)
            } else {
                None
            });

        let output_tokens = if let ReturnType::Type(_, ty) = &method_item.sig.output {
            quote!(#ty)
        } else {
            quote!(())
        };

        Ok(Self {
            id,
            hash,
            input_tokens: quote! {
                (#(#input_tys),*)
            },
            output_tokens,
        })
    }

    fn fill_with_ink_data(&self, method_item: &mut TraitItemMethod) {
        let Method { id, input_tokens, output_tokens, .. } = self;

        let input_bound = parse_quote! {
            #input_tokens: ::scale::Encode
        };

        let output_bound = parse_quote! {
            #output_tokens: ::scale::Decode
        };

        if let Some(where_clause) = &mut method_item.sig.generics.where_clause {
            where_clause.predicates.push(input_bound);
            where_clause.predicates.push(output_bound);
        } else {
            method_item.sig.generics.where_clause = Some(parse_quote! {
                where #input_bound, #output_bound
            });
        }

        let input_bindings = method_item.sig
            .inputs
            .iter()
            .filter_map(|input| if let FnArg::Typed(pat) = input {
                Some(&*pat.pat)
            } else {
                None
            });

        method_item.default = Some(parse_quote! {{
            ::obce::ink_lang::env::chain_extension::ChainExtensionMethod::build(#id)
                .input::<#input_tokens>()
                .output::<#output_tokens>()
                .ignore_error_code()
                .call(&(#(#input_bindings),*))
        }});
    }
}

pub fn generate(attrs: TokenStream, input: TokenStream) -> Result<TokenStream, Error> {
    let mut trait_item: ItemTrait = parse2(input)?;
    
    let trait_attrs = TraitAttrs::new(&trait_item, parse2(attrs)?)?;

    let trait_id = trait_attrs.id;
    let trait_name = &trait_item.ident;

    let (impls, types, where_clause) = trait_item.generics.split_for_impl();

    let methods: Vec<_> = trait_item
        .items
        .iter_mut()
        .map(|item| {
            if let TraitItem::Method(method) = item {
                Method::new(method)
            } else {
                Err(format_err_spanned!(
                    item,
                    "only methods are supported in trait definitions"
                ))
            }
        })
        .try_collect()?;

    let method_descriptions = methods
        .iter()
        .map(|Method { id, hash, input_tokens, output_tokens, .. }| quote! {
            impl #impls ::obce::codegen::MethodDescription<#hash> for dyn #trait_name #types #where_clause {
                const ID: ::core::primitive::u16 = #id;
                type Input = #input_tokens;
                type Output = #output_tokens;
            }
        });

    let mut ink_trait_item = trait_item.clone();
        
    ink_trait_item
        .items
        .iter_mut()
        .zip(methods.iter())
        .for_each(|(item, method)| if let TraitItem::Method(method_item) = item {
            method.fill_with_ink_data(method_item);
        } else {
            // FIXME: Remove unreachable call by using types?
            unreachable!("only methods are present here")
        });

    Ok(quote! {
        impl #impls ::obce::codegen::ExtensionDescription for dyn #trait_name #types #where_clause {
            const ID: ::core::primitive::u16 = #trait_id;
        }

        #(#method_descriptions)*

        #[cfg(feature = "substrate")]
        #trait_item

        #[cfg(feature = "ink")]
        #ink_trait_item
    })
}

fn find_id(attrs: &AttributeArgs) -> Result<Option<u16>, Error> {
    attrs
        .iter()
        .find_map(|arg| {
            match arg {
                NestedMeta::Meta(Meta::NameValue(value)) if value.path.is_ident("id") => {
                    Some(match &value.lit {
                        Lit::Int(lit_int) => lit_int.base10_parse::<u16>(),
                        Lit::Str(lit_str) => Ok(into_u16(lit_str.value())),
                        _ => Err(format_err_spanned!(value, "id should be integer or string")),
                    })
                }
                _ => None,
            }
        })
        .transpose()
}
