use proc_macro::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parse_macro_input};
use syn::{DeriveInput, Field, Ident, LitStr, Token};

#[derive(Debug)]
struct GbnfStructDef {
    name: Ident,
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for GbnfStructDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // let _ = Discard tokens we don't care about.
        let _: Option<Token![pub]> = input.parse()?;
        let _: Option<Token![struct]> = input.parse()?;

        let content;
        let name: Ident = input.parse()?;
        let _ = braced!(content in input);

        Ok(GbnfStructDef {
            name,
            fields: content.parse_terminated(Field::parse_named, Token![,])?,
        })
    }
}

fn generate_gbnf(input: TokenStream, create_struct: bool) -> TokenStream {
    // To define complex types, we take a struct into the macro, and
    // then output a bunch of calls to gbnf_field (wrapped in gbnf
    // complex).

    // We could also generate the entire complex type now during macro
    // run, and then shove the resulting GBNF rule into the type as a
    // static string.

    if let Ok(expr_struct) = syn::parse::<GbnfStructDef>(input) {
        let struct_name_str = LitStr::new(&expr_struct.name.to_string(), Span::call_site().into());
        let struct_name = expr_struct.name;
        let fields = expr_struct.fields.iter();

        let gbnfs = expr_struct.fields.iter().map(|field| {
            let field_type = &field.ty;
            let field_ident = field
                .ident
                .as_ref()
                .map(|i| i.to_string())
                .map(|field_name| LitStr::new(&field_name, Span::call_site().into()))
                .expect("no ident");

            quote! { gbnf_field!(#field_ident, #field_type) }
        });

        let struct_frag = if create_struct {
            quote! {
                pub struct #struct_name {
                    #(#fields),*
                }
            }
        } else {
            quote! {}
        };

        let code = quote! {
            #struct_frag

            impl #struct_name {
                pub fn to_grammar() -> &'static str {
                    use std::sync::OnceLock;
                    static GRAMMAR: OnceLock<String> = OnceLock::new();
                    GRAMMAR.get_or_init(|| Self::to_gbnf().as_complex().to_grammar())
                }
            }

            impl AsGbnf for #struct_name {
                fn to_gbnf() -> gbnf::GbnfFieldType {
                    GbnfFieldType::Complex(
                        GbnfComplex {
                            name: String::from(#struct_name_str),
                            fields: vec![#(#gbnfs),*]
                        }
                    )
                }
            }
        };

        code.into()
    } else {
        panic!("Can only generate GBNF from structs (pub or private)");
    }
}

/// Create a GBNF complex type as a Rust struct.
#[proc_macro]
pub fn gbnf_complex(input: TokenStream) -> TokenStream {
    generate_gbnf(input, true)
}

/// Add the ability to convert a Rust type into a GBNF grammar.
#[proc_macro_derive(Gbnf)]
pub fn gbnf(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    generate_gbnf(input.to_token_stream().into(), false)
}
