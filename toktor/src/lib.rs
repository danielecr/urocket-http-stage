extern crate proc_macro;
use proc_macro::TokenStream;

use quote::{quote};
use syn::{
    braced,
    parse::{Parse, ParseStream, discouraged::AnyDelimiter},
    parse_macro_input,
    punctuated::Punctuated,
    token, Attribute, Field, GenericArgument, Ident, Result, Token, Type, Visibility, WhereClause,
    WherePredicate,
};


struct StructHandle {
    actortype: Ident,
    actorhandler: Ident,
    msgtype: Ident,
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for StructHandle {
    fn parse(input: ParseStream) -> Result<Self> {
        let arguments_content;
        let _brace = braced!(arguments_content in input);
        let fields = arguments_content.parse_terminated(Field::parse_named, Token![,])?;
        let _separator: Token![=>] =  input.parse()?;
        let actortype: Ident = input.parse()?;
        let _separator: Token![,] = input.parse()?;
        let actorhandler: Ident = input.parse()?;
        let _separator: Token![,] = input.parse()?;
        let msgtype: Ident = input.parse()?;

        Ok(StructHandle {
            actortype,
            actorhandler,
            msgtype,
            fields,
        })
    }
}

#[proc_macro]
pub fn actor_handler(input: TokenStream) -> TokenStream {
    let StructHandle {
        actortype,
        actorhandler,
        msgtype,
        fields: parsed_fields,
        ..
    } = parse_macro_input!(input as StructHandle);
        
    let fields: Vec<proc_macro2::TokenStream> = parsed_fields
        .iter()
        .map(|f| {
            let Field {ident, colon_token, ty, ..} = f;
            quote! {
                #ident #colon_token #ty
            }
        }).collect();

    let params: Vec<proc_macro2::TokenStream> = parsed_fields.iter()
        .map(|f| {
            let Field {ident, ..}  = f;
            quote!{
                #ident
            }
        }).collect();

    (quote!{
        #[derive(Clone)]
        pub struct #actorhandler {
            pub sender: ::tokio::sync::mpsc::Sender<#msgtype>,
        }
        impl #actorhandler {
            pub fn new( #(#fields),* ) -> #actorhandler {
                let (sender, receiver) = ::tokio::sync::mpsc::channel(8);
                let mut actor = <#actortype>::new(receiver, #(#params),* );
                ::tokio::spawn(async move { actor.run().await; });
                #actorhandler {sender}
            }
        }
    }).into()
}

struct TokStru {
    e: Ident,
}

impl Parse for TokStru {
    fn parse(input: ParseStream) -> Result<Self> {
        return Ok(Self { e: input.parse()? })
    }
}

// see comment below
//#[proc_macro_attribute]
//pub fn toktore(args: TokenStream, input: TokenStream) -> TokenStream {
//    parse_macro_input!(input as TokStru);
//
//    let x = format!(r#"
//        fn lsls() {{
//            println!("entering");
//            println!("args tokens: {{}}", {args});
//            println!("input tokens: ");
//            println!("exiting ");
//        }}
//    "#,
//            args = args.into_iter().count(),
//            
//    );
//
//    x.parse().expect("Generated invalid tokens")
//}

