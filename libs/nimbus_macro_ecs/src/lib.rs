use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input};

/// Derive macro that marks a type as an ECS component.
///
/// The generated implementation uses the `Component` trait that lives in the
/// `nimbus_ecs` crate. Consumers typically depend on `nimbus_ecs` only and gain
/// access to this derive through its re-export.
#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let ident = ast.ident;
    let crate_path = resolve_crate_path();

    TokenStream::from(quote! {
        impl #crate_path::Component for #ident {}
    })
}

fn resolve_crate_path() -> proc_macro2::TokenStream {
    match crate_name("nimbus_ecs") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::nimbus_ecs),
    }
}
