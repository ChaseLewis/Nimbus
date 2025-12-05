use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input, ItemStruct, parse::Parse, parse::ParseStream, Token, LitInt};

/// Marks a struct as an ECS component.
///
/// By default, components are **serializable** with auto-derived `Default` and `Serialize`/`Deserialize`.
///
/// # Basic Usage (Serializable by Default)
///
/// ```ignore
/// use nimbus_ecs::component;
///
/// // Auto-derives: Serialize, Deserialize, Default
/// // Component is optional during deserialization
/// #[component]
/// struct Velocity { dx: f32, dy: f32 }
/// ```
///
/// # Custom ID
///
/// Provide a stable ID for serialization compatibility:
///
/// ```ignore
/// #[component(id = 0x1234567890ABCDEF)]
/// struct Position { x: f32, y: f32 }
/// ```
///
/// # Custom Default
///
/// Use `custom_default` when you want to provide your own `Default` implementation:
///
/// ```ignore
/// #[component(custom_default)]
/// struct Health { current: i32, max: i32 }
///
/// impl Default for Health {
///     fn default() -> Self { Self { current: 100, max: 100 } }
/// }
/// ```
///
/// # Required Components (No Default)
///
/// Use `no_default` for components that must always be provided:
///
/// ```ignore
/// #[component(no_default)]
/// struct Position { x: f32, y: f32 }  // Must be in serialized data
/// ```
///
/// # Custom Serialization
///
/// Use `custom_serialize` when you provide your own serde impls:
///
/// ```ignore
/// #[component(custom_serialize)]
/// struct CustomData { ... }
///
/// impl Serialize for CustomData { ... }
/// impl Deserialize for CustomData { ... }
/// ```
///
/// # Non-Serializable Components
///
/// Use `no_serialize` for components that should never be saved:
///
/// ```ignore
/// #[component(no_serialize)]
/// struct RuntimeCache { ... }  // Not registered, not saved
/// ```
///
/// # ZSTs (Zero-Sized Types)
///
/// Marker components are always optional (no data to serialize):
///
/// ```ignore
/// #[component]
/// struct Player;  // ZST - always optional
/// ```
///
/// # Attributes Summary
///
/// **Serialization:**
/// - `#[component]` - Auto-derives Serialize/Deserialize (default)
/// - `#[component(custom_serialize)]` - User provides serde impls
/// - `#[component(no_serialize)]` - Not serializable
///
/// **Defaults:**
/// - `#[component]` - Auto-derives Default (default)
/// - `#[component(custom_default)]` - User provides Default impl
/// - `#[component(no_default)]` - Required, no default
///
/// **Other:**
/// - `#[component(id = 0x...)]` - Explicit component ID
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let args = if attr.is_empty() {
        ComponentArgs::default()
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
    
    // Generate component ID: use provided ID or hash the fully qualified name
    let component_id = if let Some(id) = args.id {
        quote! { #crate_path::ComponentId::new(#id) }
    } else {
        // Hash: module_path + "::" + type_name
        // We use concat! to build the string at compile time in the generated code
        quote! {
            #crate_path::ComponentId::new(
                #crate_path::component::const_fnv1a_64_str(
                    concat!(module_path!(), "::", #name_str)
                )
            )
        }
    };
    
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
    
    let component_impl = quote! {
        impl #crate_path::Component for #name {
            const COMPONENT_ID: #crate_path::ComponentId = #component_id;
        }
    };
    
    match args.serialize_mode {
        SerializeMode::AutoDerive => {
            // Determine derives and registration based on default mode
            let (derives, registration) = match args.default_mode {
                DefaultMode::AutoDerive => (
                    // Auto-derive Default along with serde
                    quote! { #[derive(serde::Serialize, serde::Deserialize, Default)] },
                    quote! {
                        #crate_path::inventory::submit! {
                            #crate_path::serialization::ComponentRegistration::new_with_default::<#name>(#name_str)
                        }
                    }
                ),
                DefaultMode::Custom => (
                    // User provides Default impl, just derive serde
                    quote! { #[derive(serde::Serialize, serde::Deserialize)] },
                    quote! {
                        #crate_path::inventory::submit! {
                            #crate_path::serialization::ComponentRegistration::new_with_default::<#name>(#name_str)
                        }
                    }
                ),
                DefaultMode::None => (
                    // No Default, component is required
                    quote! { #[derive(serde::Serialize, serde::Deserialize)] },
                    quote! {
                        #crate_path::inventory::submit! {
                            #crate_path::serialization::ComponentRegistration::new::<#name>(#name_str)
                        }
                    }
                ),
            };
            
            quote! {
                #derives
                #struct_def
                
                #component_impl
                
                #registration
            }
        },
        SerializeMode::Custom => {
            // User provides serde impls, we still register for serialization
            let registration = match args.default_mode {
                DefaultMode::AutoDerive | DefaultMode::Custom => {
                    quote! {
                        #crate_path::inventory::submit! {
                            #crate_path::serialization::ComponentRegistration::new_with_default::<#name>(#name_str)
                        }
                    }
                },
                DefaultMode::None => {
                    quote! {
                        #crate_path::inventory::submit! {
                            #crate_path::serialization::ComponentRegistration::new::<#name>(#name_str)
                        }
                    }
                },
            };
            
            // For custom_serialize with auto default, we still derive Default
            let derives = match args.default_mode {
                DefaultMode::AutoDerive => quote! { #[derive(Default)] },
                _ => quote! {},
            };
            
            quote! {
                #derives
                #struct_def
                
                #component_impl
                
                #registration
            }
        },
        SerializeMode::None => {
            // Not serializable at all - just implement Component
            quote! {
                #struct_def
                
                #component_impl
            }
        },
    }.into()
}

/// How serialization is handled
#[derive(Default, Clone, Copy, PartialEq)]
enum SerializeMode {
    /// Auto-derive Serialize/Deserialize (default behavior)
    #[default]
    AutoDerive,
    /// User provides serde impls
    Custom,
    /// Not serializable
    None,
}

/// How the Default trait is handled for serializable components
#[derive(Default, Clone, Copy, PartialEq)]
enum DefaultMode {
    /// Auto-derive Default, component is optional (default behavior)
    #[default]
    AutoDerive,
    /// User provides impl Default, component is optional
    Custom,
    /// No Default, component is required
    None,
}

#[derive(Default)]
struct ComponentArgs {
    id: Option<u64>,
    serialize_mode: SerializeMode,
    default_mode: DefaultMode,
}

impl Parse for ComponentArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = ComponentArgs::default();
        
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            
            if ident == "custom_serialize" {
                args.serialize_mode = SerializeMode::Custom;
            } else if ident == "no_serialize" {
                args.serialize_mode = SerializeMode::None;
            } else if ident == "custom_default" {
                args.default_mode = DefaultMode::Custom;
            } else if ident == "no_default" {
                args.default_mode = DefaultMode::None;
            } else if ident == "id" {
                input.parse::<Token![=]>()?;
                let lit: LitInt = input.parse()?;
                args.id = Some(lit.base10_parse::<u64>().or_else(|_| {
                    // Try parsing as hex
                    let s = lit.to_string();
                    if s.starts_with("0x") || s.starts_with("0X") {
                        u64::from_str_radix(&s[2..], 16)
                            .map_err(|e| syn::Error::new(lit.span(), format!("invalid hex literal: {}", e)))
                    } else {
                        Err(syn::Error::new(lit.span(), "expected u64 literal"))
                    }
                })?);
            } else {
                return Err(syn::Error::new(ident.span(), "expected `custom_serialize`, `no_serialize`, `custom_default`, `no_default`, or `id`"));
            }
            
            // Handle optional comma between args
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        
        Ok(args)
    }
}

/// Legacy derive macro for backwards compatibility.
/// Prefer using `#[component]` attribute instead.
/// 
/// Note: This generates a TypeId-based ComponentId for backwards compatibility,
/// which is NOT stable across builds. Use `#[component]` for new code.
#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let ident = &ast.ident;
    let name_str = ident.to_string();
    let crate_path = resolve_crate_path();

    // For the legacy derive, use the same hash approach
    TokenStream::from(quote! {
        impl #crate_path::Component for #ident {
            const COMPONENT_ID: #crate_path::ComponentId = #crate_path::ComponentId::new(
                #crate_path::component::const_fnv1a_64_str(
                    concat!(module_path!(), "::", #name_str)
                )
            );
        }
    })
}

fn resolve_crate_path() -> proc_macro2::TokenStream {
    match crate_name("nimbus_ecs") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
        Err(_) => {
            // Fallback: check if we're inside nimbus_ecs by checking CARGO_PKG_NAME.
            // This handles test compilation where crate_name() may fail.
            if std::env::var("CARGO_PKG_NAME").ok().as_deref() == Some("nimbus_ecs") {
                quote!(crate)
            } else {
                quote!(::nimbus_ecs)
            }
        }
    }
}
