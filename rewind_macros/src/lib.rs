use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::Parse, parse_macro_input, parse_quote, token::Token, visit::Visit, visit_mut::VisitMut,
    Block,
};

struct IsoToArg {
    eq: syn::Token![=],
    right: syn::Path,
}
impl Parse for IsoToArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        syn::Ident::parse(&input).and_then(|t| {
            if &t.to_string() != "to" {
                Err(input.error("expecting to = ..."))
            } else {
                Ok(t)
            }
        })?;

        Ok(Self {
            eq: input.parse()?,
            right: input.parse()?,
        })
    }
}
struct IsoArgs {
    to: IsoToArg,
}
type IsoBody = syn::ItemFn;

impl Parse for IsoArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self { to: input.parse()? })
    }
}

struct SelfRenameVisitor {
    rename: Ident,
}
impl VisitMut for SelfRenameVisitor {
    fn visit_path_segment_mut(&mut self, i: &mut syn::PathSegment) {
        let rename = &self.rename;
        if i.ident == Ident::new("self", Span::call_site()) {
            *i = parse_quote! { #rename };
        }
    }
}

fn isomorphic_main(args: IsoArgs, body: IsoBody) -> TokenStream {
    let mut wrap = body.clone();

    let first_arg_ty = match body
        .sig
        .inputs
        .iter()
        .nth(0)
        .map(|f| match f {
            syn::FnArg::Receiver(r) => parse_quote! { Self },
            syn::FnArg::Typed(t) => t.ty.clone(),
        })
        .ok_or_else(|| syn::Error::new_spanned(&body.sig.inputs, "must have at least 1 argument"))
    {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };
    let second_arg_ty = body
        .sig
        .inputs
        .iter()
        .nth(1)
        .map(|f| match f {
            syn::FnArg::Receiver(_) => unreachable!(),
            syn::FnArg::Typed(t) => t.ty.to_owned(),
        })
        .unwrap_or(Box::new(parse_quote!(())));
    wrap.sig.output = parse_quote! { -> ::rewind::atom::SideEffect<(), (), #first_arg_ty, impl FnOnce(&mut #first_arg_ty, ())> };
    if let Some(syn::FnArg::Receiver(r)) = wrap.sig.inputs.first_mut() {
        let rf = r
            .reference
            .as_ref()
            .map(|(and, lifetime)| quote! { #and #lifetime });
        let mutab = &r.mutability;
        r.ty = parse_quote! { #rf #mutab ::rewind::atom::Encased<Self> };
        r.mutability.take();
        r.reference.take();
    }
    let first_arg_name = match body.sig.inputs.first().unwrap() {
        syn::FnArg::Receiver(_) => parse_quote! { self },
        syn::FnArg::Typed(t) => t.pat.to_owned(),
    };
    let mut orig_body = body.block.clone();
    let to_target = &args.to.right;
    let self_rename = Ident::new("__rewind_iso_self", Span::mixed_site());
    SelfRenameVisitor {
        rename: self_rename.clone(),
    }
    .visit_block_mut(&mut orig_body);
    wrap.block = parse_quote! { { (#first_arg_name).peel_mut(move |#self_rename| { #orig_body }, move |__rewind_iso_self, _| { __rewind_iso_self. #to_target (); }) } };
    wrap.to_token_stream()
}

#[proc_macro_attribute]
pub fn isomorphic(
    args: proc_macro::TokenStream,
    raw: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    isomorphic_main(
        parse_macro_input!(args as IsoArgs),
        parse_macro_input!(raw as IsoBody),
    )
    .into()
}
