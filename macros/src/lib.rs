use proc_macro::TokenStream;
mod all_args_constructor;
mod data;
mod getter;
mod setter;
mod no_arg_constructor;
mod accessors;
mod bean;
mod component;
mod scope;



#[proc_macro_attribute]
pub fn data(attribute: TokenStream, item: TokenStream) -> TokenStream {
    data::data_impl(attribute, item)
}

#[proc_macro_attribute]
pub fn getter(attribute: TokenStream, item: TokenStream) -> TokenStream {
     getter::getter_impl(attribute, item)
}

#[proc_macro_attribute]
pub fn setter(attribute: TokenStream, item: TokenStream) -> TokenStream {
    setter::setter_impl(attribute, item)
}

#[proc_macro_attribute]
pub fn no_arg_constructor(attribute: TokenStream, item: TokenStream) -> TokenStream {
    no_arg_constructor::no_arg_constructor_impl(attribute, item)
}

#[proc_macro_attribute]
pub fn all_args_constructor(attribute: TokenStream, item: TokenStream) -> TokenStream {
    all_args_constructor::all_args_constructor_impl(attribute, item)
}

#[proc_macro_attribute]
pub fn bean(attribute: TokenStream, item: TokenStream) -> TokenStream {
    bean::bean_impl(attribute, item)
}
#[proc_macro_attribute]
pub fn component(attribute: TokenStream, item: TokenStream) -> TokenStream {
    component::component_impl(attribute, item)
}