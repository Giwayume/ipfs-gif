extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{ format_ident, quote };
use syn::{
    parse_macro_input, Data, Expr, LitStr, Token,
    punctuated::Punctuated,
};

#[proc_macro_derive(RouteParamsContext, attributes(route_param_source))]
pub fn derive_into_hash_map(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);

    let struct_identifier = &input.ident;

    let expanded = match &input.data {
        Data::Struct(syn::DataStruct { fields, .. }) => {
            let mut implementation = quote!{};

            let mut needs_struct_default = false;

            for field in fields {
                let identifier = field.ident.as_ref().unwrap();
                let field_type = &field.ty;
                let attrs = &field.attrs;

                let mut source_parsed: Option<String> = None;
                let mut name_parsed: Option<String> = None;
                let mut default_parsed = None;

                for attr in attrs {
                    if attr.path().is_ident("route_param_source") {
                        let _ = attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident("source") {
                                meta.input.parse::<Token![=]>()?;
                                let lit: LitStr = meta.input.parse()?;
                                source_parsed = Some(lit.value());
                            }
                            if meta.path.is_ident("name") {
                                meta.input.parse::<Token![=]>()?;
                                let lit: LitStr = meta.input.parse()?;
                                name_parsed = Some(lit.value());
                            }
                            if meta.path.is_ident("default") {
                                meta.input.parse::<Token![=]>()?;
                                let lit: LitStr = meta.input.parse()?;
                                default_parsed = Some(lit.value());
                            }
                            Ok(())
                        });
                    }
                }
                
                let source = source_parsed.unwrap_or_else(|| String::from("query"));
                let source_ident = format_ident!("{}", source);
                let parse_name = name_parsed.unwrap_or_else(|| identifier.to_string());
                let unwrap_ident = if default_parsed.is_some() {
                    quote!{.unwrap_or_else(|| {
                        #default_parsed.parse::<#field_type>().unwrap()
                    })}
                } else {
                    quote!{.unwrap_or_else(|| #field_type::default())}
                };

                if source == "query" || source == "path" || source == "form" {
                    implementation.extend(quote!{
                        #identifier: #source_ident.get(#parse_name).and_then(|value| value.parse::<#field_type>().ok())#unwrap_ident,
                    });
                } else if source == "none" {
                    if default_parsed.is_some() {
                        implementation.extend(quote!{
                            #identifier: #default_parsed.parse::<#field_type>().unwrap(),
                        });
                    } else {
                        needs_struct_default = true
                    }
                }
            }

            if needs_struct_default {
                implementation.extend(quote!{
                    ..Default::default()
                })
            }

            quote! {
                #[automatically_derived]
                impl RouteParamContextGenerator for #struct_identifier {
                    type Type = #struct_identifier;

                    fn populate_from_context_extractor(
                        path: &std::collections::HashMap<String, String>,
                        query: &std::collections::HashMap<String, String>,
                        form: &std::collections::HashMap<String, String>,
                    ) -> Self::Type {
                        #struct_identifier {
                            #implementation
                        }
                    }
                }
            }
        }
        _ => unimplemented!()
    };

    // println!("{}", expanded.to_string());

    expanded.into()
}

#[proc_macro]
pub fn render_template(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input with Punctuated::<Expr, Token![,]>::parse_terminated);

    let mut iter = input.iter();
    let template_expr = iter.next().expect("Expected template expression");
    let context_expr = iter.next().expect("Expected context expression");
    let block_expr = iter.next(); // Option<&Expr>

    let template_render_call = match block_expr {
        Some(e) => {
            let ident = match e {
                Expr::Path(p) if p.path.segments.len() == 1 => p.path.segments[0].ident.clone(),
                _ => panic!("3rd argument must be an identifier, e.g. `block_name`"),
            };

            let block_method = quote::format_ident!("as_{}", ident);
            // Note: method call is `template.<method>.render()`
            quote! {
                match template.#block_method().render() {
                    Ok(html) => Ok(html),
                    Err(e) => Err(crate::util::error::RenderingError::from(e)),
                }
            }
        }
        None => {
            quote! {
                match template.render() {
                    Ok(html) => Ok(html),
                    Err(e) => Err(crate::util::error::RenderingError::from(e)),
                }
            }
        }
    };

    let expanded = quote! {
        match #template_expr::new(#context_expr).await {
            Ok(template) => {
                match std::panic::catch_unwind(|| {
                    #template_render_call
                }) {
                    Ok(html) => html,
                    Err(e) => Err(crate::util::error::RenderingError::from(e)),
                }
            },
            Err(e) => Err(crate::util::error::RenderingError::from(e)),
        }
    };

    TokenStream::from(expanded)
}
