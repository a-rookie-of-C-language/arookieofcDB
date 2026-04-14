use proc_macro2::Span;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Field, Ident, Type};

pub fn build_getter_methods(
    fields: &Punctuated<Field, Comma>,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut methods = Vec::new();
    for field in fields {
        let field_ident = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(field, "field must have an identifier"))?;
        let field_name = field_ident.to_string();
        let field_type = &field.ty;
        let getter_name = Ident::new(&format!("get_{}", field_name), Span::call_site());
        
        let return_type = match field_type {
            Type::Path(path) => {
                let type_str = path.path.segments.last().unwrap().ident.to_string();
                match type_str.as_str() {
                    "String" => quote! { &str },
                    "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" | "i128" | "f32" | "f64" | "bool" => quote! { #field_type },
                    _ => quote! { &#field_type },
                }
            }
            _ => quote! { &#field_type },
        };
        
        let body = match field_type {
            Type::Path(path) => {
                let type_str = path.path.segments.last().unwrap().ident.to_string();
                match type_str.as_str() {
                    "String" => quote! { self.#field_ident.as_str() },
                    "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" | "i128" | "f32" | "f64" | "bool" => quote! { self.#field_ident },
                    _ => quote! { &self.#field_ident },
                }
            }
            _ => quote! { &self.#field_ident },
        };
        
        methods.push(quote! {
            pub fn #getter_name(&self) -> #return_type {
                #body
            }
        });
    }
    Ok(methods)
}

pub fn build_setter_methods(
    fields: &Punctuated<Field, Comma>,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut methods = Vec::new();
    for field in fields {
        let field_ident = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(field, "field must have an identifier"))?;
        let field_name = field_ident.to_string();
        let field_type = &field.ty;
        let setter_name = Ident::new(&format!("set_{}", field_name), Span::call_site());
        methods.push(quote! {
            pub fn #setter_name(&mut self, value: #field_type) {
                self.#field_ident = value;
            }
        });
    }
    Ok(methods)
}
