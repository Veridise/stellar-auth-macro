//! Proc macros:
//! - #[access_control] on an `impl` block (or inline `mod`):
//!     Emits compile errors if any public `fn` inside is missing #[no_access_control].
//! - #[no_access_control] on a function:
//!     Marker attribute (no-op) to indicate the fn is allowed.

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    Attribute, ImplItem, Item, ItemImpl, ItemMod, Visibility,
};

fn has_no_access_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("no_access_control"))
}

/// Marker attribute for functions that are allowed (no-op).
#[proc_macro_attribute]
pub fn no_access_control(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Place on an `impl` block (or inline `mod`). Errors if any public `fn`
/// lacks #[no_access_control].
#[proc_macro_attribute]
pub fn access_control(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Try `impl` first
    if let Ok(impl_block) = syn::parse::<ItemImpl>(item.clone()) {
        let mut errors = Vec::new();

        for it in &impl_block.items {
            if let ImplItem::Fn(m) = it {
                // Consider anything not-private as requiring the marker:
                // (pub, pub(crate), pub(super), pub(in ...))
                let is_publicish = !matches!(m.vis, Visibility::Inherited);
                if is_publicish && !has_no_access_attr(&m.attrs) {
                    let name = &m.sig.ident;
                    errors.push(quote_spanned! { m.sig.ident.span()=>
                        compile_error!(concat!(
                            "missing #[no_access_control] on public method: ",
                            stringify!(#name)
                        ));
                    });
                }
            }
        }

        if errors.is_empty() {
            // No problems: return the original item
            return item;
        } else {
            // Emit compile errors + original item for context
            let orig = impl_block.into_token_stream();
            return TokenStream::from(quote! { #(#errors)* #orig });
        }
    }

    // Also support inline modules: `#[access_control] mod m { pub fn ... }`
    if let Ok(module) = syn::parse::<ItemMod>(item.clone()) {
        if let Some((_, items)) = &module.content {
            let mut errors = Vec::new();

            for it in items {
                if let Item::Fn(f) = it {
                    let is_publicish = !matches!(f.vis, Visibility::Inherited);
                    if is_publicish && !has_no_access_attr(&f.attrs) {
                        let name = &f.sig.ident;
                        errors.push(quote_spanned! { f.sig.ident.span()=>
                            compile_error!(concat!(
                                "missing #[no_access_control] on public function: ",
                                stringify!(#name)
                            ));
                        });
                    }
                }
            }

            if errors.is_empty() {
                return item;
            } else {
                let orig = module.into_token_stream();
                return TokenStream::from(quote! { #(#errors)* #orig });
            }
        }

        // External module file; we can't inspect inside. Leave it unchanged.
        // @todo Throw an error. Since we can't inspect it, we ideally want to enforce that the macro
        // can’t be used on these types of modules
        return item;
    }

    // Wrong placement
    TokenStream::from(quote! {
        compile_error!("#[access_control] must be placed on an `impl` block or an inline `mod`.");
    })
}
