use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{DataStruct, DeriveInput, parse_macro_input};

#[proc_macro_derive(FromJsonObject)]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse_macro_input!(input);

    let struct_name = ast.ident;
    let struct_fields = match ast.data {
        syn::Data::Struct(DataStruct {
            fields: syn::Fields::Named(fields),
            ..
        }) => fields
            .named
            .into_iter()
            .map(|f| (f.ident.unwrap(), f.ty))
            .collect::<Vec<_>>(),
        _ => unimplemented!(),
    };
    let field_names = struct_fields.iter().map(|t| &t.0).collect::<Vec<_>>();
    let field_types = struct_fields.iter().map(|t| &t.1).collect::<Vec<_>>();

    let match_expr = quote! {
        #( map.remove(stringify!(#field_names)).and_then(<#field_types>::from_json) ),*
    };

    let match_case = quote! {
        #( Some(#field_names) ),*
    };

    let output = quote! {
        #[automatically_derived]
        impl FromJson for #struct_name {    
            fn from_json(json: Json) -> Option<Self> {
                match json {
                    Json::Object(mut map) => match (#match_expr) {
                        (#match_case) => Some( Self {
                            #( #field_names ),*
                        }),
                        _ => None,
                    },
                    _ => None,
                }
            }
        }
    };

    output.into()
}
