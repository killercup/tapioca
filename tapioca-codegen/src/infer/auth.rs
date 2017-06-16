use ::inflector::Inflector;
use ::quote::Tokens;
use ::syn::Ident;
use ::yaml_rust::Yaml;

use infer::StructBoundArgImpl;
use infer::TokensResult;

fn infer_v3_http(scheme_ident: &Ident, schema: &Yaml) -> TokensResult {
    match schema["scheme"].as_str().expect("http security scheme must be a string")
        .to_title_case().as_str()
    {
        "Basic" => Ok(quote! {
            pub type #scheme_ident = header::Basic;

            impl From<(String, String)> for #scheme_ident {
                fn from((username, password): &(String, String)) -> Self {
                    Self { username, password }
                }
            }
        }),
        _ => Err(From::from("currently supported HTTP auth schemes are: Basic")),
    }
}

pub(super) fn infer_v3_component(scheme_name: &str, schema: &Yaml) -> TokensResult {
    let ident = Ident::from(scheme_name.to_class_case());

    match schema["type"].as_str().expect("security scheme type must be a string")
        .to_camel_case().as_str()
    {
        "http" => infer_v3_http(&ident, &schema),
        _ => Err(From::from("currently supported auth types are: http")),
    }
}

pub(super) fn infer_v3(structs_mod: &Ident, schema: &Yaml) -> StructBoundArgImpl {
    match schema.as_hash() {
        Some(security_schemes) => {
            let mut scheme_variants: Vec<Ident> = Vec::new();
            let mut scheme_models: Vec<Tokens> = Vec::new();
            let mut scopes_models: Vec<Tokens> = Vec::new();

            for (scheme, scopes) in security_schemes {
                let classname = scheme.as_str().expect("security scheme must be a string")
                    .to_class_case();
                let ident = Ident::from(classname);

                scheme_variants.push(ident.clone());
                scheme_models.push(quote!{ auth_scheme::#ident });

                let mut scopes_model = quote!();
                for scope in scopes.as_vec().expect("scope must be an array") {
                    let classname = scope.as_str().expect("scope must be a string")
                        .to_class_case();
                    let ident = Ident::from(classname);

                    scopes_model.append(quote!{ auth_scheme::scope::#ident, });
                }

                scopes_models.push(scopes_model);
            }

             Ok((
                quote! {
                    pub enum OperationAuth {
                        #(#scheme_variants(#scheme_models<#scopes_models>),)*
                    }
                },
                quote!(),
                quote!(authentication: &#structs_mod::OperationAuth),
                quote! {
                    .header(authentication)
                }
            ))
        },
        None => Ok((
            quote!(),
            quote!(),
            quote!(authentication: &ServerAuth),
            quote! {
                .header(authentication)
            }
        )),
    }

}