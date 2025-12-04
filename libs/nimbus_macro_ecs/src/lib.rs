use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input, ItemStruct, parse::Parse, parse::ParseStream};

/// Marks a struct as an ECS component.
///
/// # Basic Usage
///
/// ```ignore
/// use nimbus_ecs::component;
///
/// #[component]
/// struct Position {
///     x: f32,
///     y: f32,
/// }
/// ```
///
/// # Serializable Components
///
/// Add `serializable` to enable automatic serialization registration:
///
/// ```ignore
/// #[component(serializable)]
/// struct Health {
///     current: f32,
///     max: f32,
/// }
/// ```
///
/// This automatically:
/// - Derives `serde::Serialize` and `serde::Deserialize`
/// - Registers the component with the global serialization registry
///
/// # Attributes
///
/// - `#[component]` - Basic component, not serializable
/// - `#[component(serializable)]` - Serializable component with auto-registration
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let args = if attr.is_empty() {
        ComponentArgs { serializable: false }
    } else {
        parse_macro_input!(attr as ComponentArgs)
    };
    
    let name = &input.ident;
    let name_str = name.to_string();
    let vis = &input.vis;
    let attrs = &input.attrs;
    let generics = &input.generics;
    let fields = &input.fields;
    let crate_path = resolve_crate_path();
    
    let struct_def = match fields {
        syn::Fields::Named(fields) => quote! {
            #(#attrs)*
            #vis struct #name #generics #fields
        },
        syn::Fields::Unnamed(fields) => quote! {
            #(#attrs)*
            #vis struct #name #generics #fields;
        },
        syn::Fields::Unit => quote! {
            #(#attrs)*
            #vis struct #name #generics;
        },
    };
    
    if args.serializable {
        quote! {
            #[derive(serde::Serialize, serde::Deserialize)]
            #struct_def
            
            impl #crate_path::Component for #name {}
            
            #crate_path::inventory::submit! {
                #crate_path::serialization::ComponentRegistration::new::<#name>(#name_str)
            }
        }
    } else {
        quote! {
            #struct_def
            
            impl #crate_path::Component for #name {}
        }
    }.into()
}

struct ComponentArgs {
    serializable: bool,
}

impl Parse for ComponentArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        if ident == "serializable" {
            Ok(ComponentArgs { serializable: true })
        } else {
            Err(syn::Error::new(ident.span(), "expected `serializable`"))
        }
    }
}

/// Legacy derive macro for backwards compatibility.
/// Prefer using `#[component]` attribute instead.
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
