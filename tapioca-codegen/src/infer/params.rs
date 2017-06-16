use ::inflector::Inflector;
use ::quote::Tokens;
use ::syn::Ident;
use ::yaml_rust::Yaml;

use infer::datatype;
use infer::StructBoundArgImpl;

pub(super) fn infer_v3(method: &str, schema: &Yaml) -> StructBoundArgImpl {
    let method_mod = Ident::new(method.to_snake_case());
    let method = method.to_uppercase();

    let mut idents: Vec<Ident> = Vec::new();
    let mut types: Vec<Tokens> = Vec::new();
    let mut supporting_types: Vec<Tokens> = Vec::new();
    let mut placeholders: Vec<String> = Vec::new();

    for schema in schema.as_vec().unwrap() {
        let name = schema["name"].as_str()
            .expect("Parameter name must be a string.");
        let (param_type, maybe_at) = datatype::infer_v3(&schema["schema"])?;

        let struct_ident = Ident::new(format!("__ResourceId_{}", name));
        supporting_types.push(quote! {
            #[allow(non_camel_case_types)]
            pub struct #struct_ident(::tapioca::datatype::Required<#param_type>);

            impl ToString for #struct_ident {
                fn to_string(&self) -> String {
                    self.0.to_string()
                }
            }
        });

        idents.push(Ident::new(name));
        types.push(quote!{ #method_mod::#struct_ident });
        placeholders.push(format!("{{{}}}", name));

        if let Some(supporting_type) = maybe_at {
            supporting_types.push(supporting_type);
        }
    }

    let params = idents.clone();

    let endpoint_id_arg = match (method.as_str(), idents.pop(), types.pop()) {
        ("DELETE", Some(endp_ident), Some(endp_type))
            // The resource ID value is moved here, to avoid its reuse
            // !FIXME: this assumes that the DELETE request succeeds
            => quote!(#endp_ident: #endp_type),

        (_, Some(endp_ident), Some(endp_type))
            // We take a reference to the ID, as for any others if nested
            => quote!(#endp_ident: &#endp_type),

        (_, None, _)
        | (_, _, None) => panic!("params::infer called without any params to infer"),
    };

    Ok((
        quote!{ #(#supporting_types)* },
        quote!(),
        quote!{ #(#idents: &#types,)* #endpoint_id_arg },
        quote! {
            .path_segments_mut().unwrap()
                .clear()
                .push(Url::parse(self::API_URL).unwrap().path())
                .extend(self::API_PATH.split('/').map(|p| match p {
                    #(#placeholders => #params.to_string(),)*
                    _ => p.to_string(),
                }))
        }
    ))
}
