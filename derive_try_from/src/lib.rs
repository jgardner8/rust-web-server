use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{DataStruct, DeriveInput, parse_macro_input};

#[proc_macro_derive(TryFromParameters)]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse_macro_input!(input);

    let struct_name = ast.ident;
    let struct_fields = match ast.data {
        syn::Data::Struct(DataStruct {
            fields: syn::Fields::Named(fields),
            ..
        }) => fields
            .named
            .iter()
            .map(|f| f.ident.clone().unwrap())
            .collect::<Vec<_>>(),
        _ => unimplemented!(),
    };

    let output = quote! {
        #[automatically_derived]
        impl TryFrom<http_server::web_server::Parameters> for #struct_name {
            type Error = http_server::web_server::StatusCode;
            fn try_from(params: http_server::web_server::Parameters) -> Result<Self, Self::Error> {
                match ( #( params.get(stringify!(#struct_fields)) ),* ) {
                    ( #( Some(#struct_fields) ),* ) => Ok(#struct_name {
                        #( #struct_fields: #struct_fields.clone() ),*
                    }),
                    _ => Err(http_server::web_server::StatusCode::BadRequest),
                }
            }
        }
    };
    
    output.into()
}
