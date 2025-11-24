//! Proc macros:
//! - #[access_control] on an `impl` block:
//!     * Instruments methods marked with #[authorized_by(...)] by injecting guards
//!       directly into their bodies, and removes the attribute so it isn't forwarded.
//!     * Emits compile errors if any public fn is missing #[no_access_control]
//!       or #[authorized_by(...)].
//! - #[no_access_control] on a function: marker (no-op).
//! - #[authorized_by(arg_ident, check_fn_or_path)] on a function:
//!     * If applied directly to a function/impl method, injects the guard.
//!     * If it lands on a generated wrapper or non-function item, it no-ops (warns at most).

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, FnArg, ImplItem, ImplItemFn, ItemFn, ItemImpl, Meta, Pat, Path, Token, Type,
    Visibility,
};

use proc_macro_error::{abort, abort_if_dirty, emit_error, emit_warning, proc_macro_error};

fn has_no_access_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("no_access_control"))
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn no_access_control(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

struct AuthorizedArgs {
    arg: syn::Ident,
    check_fn: Path,
}

/// The parser enforces the format: `<ident> , <path>`.
impl Parse for AuthorizedArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let arg: syn::Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let check_fn: Path = input.parse()?;
        Ok(Self { arg, check_fn })
    }
}

/// Iterates over the function signature's arguments and returns the "desired" parameter, if it exists.
fn find_param_ident(sig: &syn::Signature, desired: &str) -> Option<syn::Ident> {
    sig.inputs.iter().find_map(|arg| match arg {
        FnArg::Typed(pat_ty) => match &*pat_ty.pat {
            Pat::Ident(p) if p.ident == desired => Some(p.ident.clone()),
            _ => None,
        },
        FnArg::Receiver(_) => None, // skip for 'self'
    })
}

/// Returns true if desired parameter exists within the function signature.
fn param_exists(sig: &syn::Signature, desired: &syn::Ident) -> bool {
    find_param_ident(sig, &desired.to_string()).is_some()
}

/// Return the parameter identifiers whose *spelled* type is `Env` (or `&Env`, or `soroban_sdk::Env`).
fn env_type_candidates(sig: &syn::Signature) -> Vec<syn::Ident> {
    let mut out = Vec::new();
    for arg in &sig.inputs {
        let FnArg::Typed(pat_ty) = arg else { continue };
        let Pat::Ident(pat_ident) = &*pat_ty.pat else { continue };

        // peel references like &Env
        let mut ty: &Type = &*pat_ty.ty;
        if let Type::Reference(r) = ty {
            ty = &*r.elem;
        }
        if let Type::Path(p) = ty {
            if let Some(seg) = p.path.segments.last() {
                if seg.ident == "Env" {
                    out.push(pat_ident.ident.clone());
                }
            }
        }
    }
    out
}

/// Hybrid Env resolution:
/// 1) If a parameter literally named `env` exists, use it.
/// 2) Else, if exactly one parameter has type `Env` (by spelled name), use it.
/// 3) Else, return None (caller may warn/error).
fn find_env_ident_hybrid(sig: &syn::Signature) -> Option<syn::Ident> {
    if let Some(id) = find_param_ident(sig, "env") {
        return Some(id);
    }
    let cands = env_type_candidates(sig);
    if cands.len() == 1 {
        Some(cands[0].clone())
    } else {
        None
    }
}

/// Returns a boxed block that:
/// 1) runs the predicate,
/// 2) calls `require_auth()` on the subject,
/// 3) then executes the original body.
fn instrument_block(
    body: &syn::Block,
    call_path: TokenStream2,
    env_ident: &syn::Ident,
    arg_ident: &syn::Ident,
    span: Span,
) -> Box<syn::Block> {
    syn::parse_quote_spanned! { span =>
        {
            if !(#call_path(&#env_ident, &#arg_ident)) {
                ::core::panic!(concat!(
                    "unauthorized: ",
                    stringify!(#call_path),
                    "(env,",
                    stringify!(#arg_ident),
                    ") failed"
                ));
            }
            #arg_ident.require_auth();
            #body
        }
    }
}

/// 1) Finds and removes the first #[authorized_by(...)] in attrs,
/// 2) parses it into AuthorizedArgs,
/// 3) returns Some(args) (or None if malformed attr).
fn take_authorized_args(attrs: &mut Vec<Attribute>) -> Option<AuthorizedArgs> {
    let idx = attrs.iter().position(|a| a.path().is_ident("authorized_by"))?;
    let attr = attrs.remove(idx);
    match attr.meta {
        Meta::List(_) => match attr.parse_args::<AuthorizedArgs>() {
            Ok(a) => Some(a),
            Err(e) => {
                emit_error!(attr.span(), "malformed #[authorized_by(...)] args: {}", e);
                None
            }
        },
        _ => {
            emit_error!(
                attr.span(),
                "#[authorized_by] must be written as #[authorized_by(arg_ident, path)]"
            );
            None
        }
    }
}

fn build_call_path(check_fn: &Path, use_self: bool) -> TokenStream2 {
    if use_self && check_fn.segments.len() == 1 {
        let ident = &check_fn.segments[0].ident;
        quote! { Self::#ident }
    } else {
        let p = check_fn;
        quote! { #p }
    }
}

fn get_env_ident_or_warn(sig: &syn::Signature, fn_name: &syn::Ident) -> Option<syn::Ident> {
    if let Some(id) = find_env_ident_hybrid(sig) {
        return Some(id);
    }
    let cands = env_type_candidates(sig);
    if cands.len() > 1 {
        emit_warning!(
            sig.span(),
            "skipping #[authorized_by]: multiple `Env`-typed parameters on `{}`; \
             please name the desired one `env`",
            fn_name
        );
    } else {
        emit_warning!(
            sig.span(),
            "skipping #[authorized_by]: no `Env` parameter found on `{}`; leaving unchanged",
            fn_name
        );
    }
    None
}

fn ensure_param_or_warn(sig: &syn::Signature, fn_name: &syn::Ident, desired: &syn::Ident) -> bool {
    if param_exists(sig, desired) {
        return true;
    }
    emit_warning!(
        desired.span(),
        "skipping #[authorized_by]: parameter `{}` not found on `{}` (generated wrapper?)",
        desired,
        fn_name
    );
    false
}

///
fn instrument_impl_like(
    sig: &syn::Signature,
    block: &mut syn::Block,
    fn_name: &syn::Ident,
    args: &AuthorizedArgs,
    use_self: bool,
) -> bool {
    if !ensure_param_or_warn(sig, fn_name, &args.arg) {
        return false;
    }
    let env_ident = match get_env_ident_or_warn(sig, fn_name) {
        Some(e) => e,
        None => return false,
    };
    let call_path = build_call_path(&args.check_fn, use_self);
    let arg = &args.arg;
    let body = &*block; // borrow before replace
    *block = *instrument_block(body, call_path, &env_ident, arg, sig.span());
    true
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn authorized_by(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = match syn::parse::<AuthorizedArgs>(attr) {
        Ok(a) => a,
        Err(e) => {
            emit_error!(e.span(), "malformed #[authorized_by(..)]: {}", e);
            return item;
        }
    };

    // impl method path
    if let Ok(mut m) = syn::parse::<ImplItemFn>(item.clone()) {
        let _ = instrument_impl_like(&m.sig, &mut m.block, &m.sig.ident, &args, /*use_self=*/ true);
        return TokenStream::from(quote!(#m));
    }

    // free function path
    if let Ok(mut f) = syn::parse::<ItemFn>(item.clone()) {
        let _ = instrument_impl_like(&f.sig, &mut f.block, &f.sig.ident, &args, /*use_self=*/ false);
        return TokenStream::from(quote!(#f));
    }

    // Not a function/method → no-op
    item
}

/// Apply to an `contractimpl` block.
/// Instruments #[authorized_by(...)] in place and removes the attribute;
/// then enforces that public functions have either #[no_access_control]
/// or #[authorized_by(...)].
#[proc_macro_error]
#[proc_macro_attribute]
pub fn access_control(_attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(mut impl_block) = syn::parse::<ItemImpl>(item.clone()) {
        for it in &mut impl_block.items {
            if let ImplItem::Fn(m) = it {
                // instrument & strip #[authorized_by(...)] if present
                let mut had_authorized = false;
                if let Some(args) = take_authorized_args(&mut m.attrs) {
                    if instrument_impl_like(&m.sig, &mut m.block, &m.sig.ident, &args, true) {
                        had_authorized = true;
                    }
                }

                let is_trait_impl = impl_block.trait_.is_some();
                let has_contractimpl_attr = impl_block
                    .attrs
                    .iter()
                    .any(|a| a.path().is_ident("contractimpl"));

                let is_public = is_trait_impl
                    || has_contractimpl_attr
                    || !matches!(m.vis, Visibility::Inherited);

                let has_no_access = has_no_access_attr(&m.attrs);

                if is_public && !(had_authorized || has_no_access) {
                    emit_error!(
                        m.sig.ident.span(),
                        "public method {} is missing #[no_access_control] or #[authorized_by(...)]",
                        m.sig.ident
                    );
                }
            }
        }
        abort_if_dirty();
        return TokenStream::from(quote!(#impl_block));
    }

    abort!(
        Span::call_site(),
        "#[access_control] must be placed on an `impl` block."
    );
}
