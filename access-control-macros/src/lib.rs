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
    Attribute, FnArg, ImplItem, ImplItemFn, Item, ItemFn, ItemImpl, Meta, Pat, Path, Token, Type,
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
impl Parse for AuthorizedArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let arg: syn::Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let check_fn: Path = input.parse()?;
        Ok(Self { arg, check_fn })
    }
}

fn ident_eq(a: &syn::Ident, b: &syn::Ident) -> bool {
    a.to_string() == b.to_string()
}

fn param_exists(sig: &syn::Signature, want: &syn::Ident) -> bool {
    sig.inputs.iter().any(|arg| match arg {
        FnArg::Typed(pat_ty) => {
            if let Pat::Ident(p) = &*pat_ty.pat {
                ident_eq(&p.ident, want)
            } else {
                false
            }
        }
        FnArg::Receiver(_) => false,
    })
}

fn find_param_ident(sig: &syn::Signature, name: &str) -> Option<syn::Ident> {
    for arg in &sig.inputs {
        if let FnArg::Typed(pat_ty) = arg {
            if let Pat::Ident(pat) = &*pat_ty.pat {
                if pat.ident == name {
                    return Some(pat.ident.clone());
                }
            }
        }
    }
    None
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

fn try_instrument_method(m: &mut ImplItemFn, args: &AuthorizedArgs) -> Option<()> {
    if !param_exists(&m.sig, &args.arg) {
        emit_warning!(
            args.arg.span(),
            "skipping #[authorized_by]: parameter `{}` not found on `{}` (generated wrapper?)",
            args.arg,
            m.sig.ident
        );
        return None;
    }
    let env_ident = match find_env_ident_hybrid(&m.sig) {
        Some(id) => id,
        None => {
            // See if we can give a better hint (ambiguous vs missing) for ergonomics
            let cands = env_type_candidates(&m.sig);
            if cands.len() > 1 {
                emit_warning!(
                    m.sig.span(),
                    "skipping #[authorized_by]: multiple `Env`-typed parameters on `{}`; \
                     please name the desired one `env`",
                    m.sig.ident
                );
            } else {
                emit_warning!(
                    m.sig.span(),
                    "skipping #[authorized_by]: no `Env` parameter found on `{}`; leaving unchanged",
                    m.sig.ident
                );
            }
            return None;
        }
    };
    let call_path = if args.check_fn.segments.len() == 1 {
        let ident = &args.check_fn.segments[0].ident;
        quote! { Self::#ident }
    } else {
        let p = &args.check_fn;
        quote! { #p }
    };
    let arg = &args.arg;
    let body = &m.block;
    m.block = *instrument_block(body, call_path, &env_ident, arg, m.sig.span());
    Some(())
}

/// Standalone attribute. Never errors on placement: falls back to no-op
/// so rust-analyzer expansion order doesn't spam diagnostics.
#[proc_macro_error]
#[proc_macro_attribute]
pub fn authorized_by(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse args; if malformed, that's a real error.
    let args = match syn::parse::<AuthorizedArgs>(attr) {
        Ok(a) => a,
        Err(e) => {
            emit_error!(e.span(), "malformed #[authorized_by(..)]: {}", e);
            return item;
        }
    };

    // Case 1: method inside an `impl`
    if let Ok(mut m) = syn::parse::<ImplItemFn>(item.clone()) {
        // only instrument if both `env` and requested param exist
        if let Some(env_ident) = find_env_ident_hybrid(&m.sig) {
            if param_exists(&m.sig, &args.arg) {
                let call_path = if args.check_fn.segments.len() == 1 {
                    let ident = &args.check_fn.segments[0].ident;
                    quote! { Self::#ident }
                } else {
                    let p = &args.check_fn;
                    quote! { #p }
                };
                let arg = &args.arg;
                let body = &m.block;
                m.block = *instrument_block(body, call_path, &env_ident, arg, m.sig.span());
            } else {
                // param not found – leave unchanged (avoid RA errors)
                emit_warning!(
                    args.arg.span(),
                    "skipping #[authorized_by]: parameter `{}` not found on `{}`; leaving unchanged",
                    args.arg, m.sig.ident
                );
            }
        } else {
            let cands = env_type_candidates(&m.sig);
            if cands.len() > 1 {
                emit_warning!(
                    m.sig.span(),
                    "skipping #[authorized_by]: multiple `Env`-typed parameters on `{}`; \
                     please name the desired one `env`",
                    m.sig.ident
                );
            } else {
                emit_warning!(
                    m.sig.span(),
                    "skipping #[authorized_by]: no `Env` parameter on `{}`; leaving unchanged",
                    m.sig.ident
                );
            }
        }
        return TokenStream::from(quote!(#m));
    }

    // Case 2: free function
    if let Ok(mut f) = syn::parse::<ItemFn>(item.clone()) {
        if let Some(env_ident) = find_env_ident_hybrid(&f.sig) {
            if param_exists(&f.sig, &args.arg) {
                let call_path = {
                    let p = &args.check_fn;
                    quote! { #p }
                };
                let arg = &args.arg;
                let body = &f.block;
                f.block = instrument_block(body, call_path, &env_ident, arg, f.sig.span());
            } else {
                emit_warning!(
                    args.arg.span(),
                    "skipping #[authorized_by]: parameter `{}` not found on function `{}`; leaving unchanged",
                    args.arg, f.sig.ident
                );
            }
        } else {
            let cands = env_type_candidates(&f.sig);
            if cands.len() > 1 {
                emit_warning!(
                    f.sig.span(),
                    "skipping #[authorized_by]: multiple `Env`-typed parameters on `{}`; \
                     please name the desired one `env`",
                    f.sig.ident
                );
            } else {
                emit_warning!(
                    f.sig.span(),
                    "skipping #[authorized_by]: no `Env` parameter on `{}`; leaving unchanged",
                    f.sig.ident
                );
            }
        }
        return TokenStream::from(quote!(#f));
    }

    // Not a function/method → just return unchanged (no error).
    item
}

/// Apply to an `impl` block.
/// Instruments #[authorized_by(...)] in place and removes the attribute;
/// then enforces that public functions have either #[no_access_control]
/// or #[authorized_by(...)].
#[proc_macro_error]
#[proc_macro_attribute]
pub fn access_control(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // impl block path
    if let Ok(mut impl_block) = syn::parse::<ItemImpl>(item.clone()) {
        for it in &mut impl_block.items {
            if let ImplItem::Fn(m) = it {
                // instrument & strip #[authorized_by(...)] if present
                let mut had_authorized = false;
                if let Some(args) = take_authorized_args(&mut m.attrs) {
                    if try_instrument_method(m, &args).is_some() {
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
