extern crate proc_macro;

mod macros {
    pub mod bundle;
    pub mod filter;
    pub mod system_param;
    pub mod query_data;
}

use proc_macro::TokenStream;

#[proc_macro_derive(ComponentBundle)]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    macros::bundle::derive_bundle_impl(input)
}

#[proc_macro_derive(QueryFilter)]
pub fn derive_filter(input: TokenStream) -> TokenStream {
    macros::filter::derive_filter_impl(input)
}

#[proc_macro_derive(QueryData)]
pub fn derive_world_query(input: TokenStream) -> TokenStream {
    macros::query_data::derive_query_data_impl(input)
}

#[proc_macro_derive(SystemParam)]
pub fn derive_system_param(input: TokenStream) -> TokenStream {
    macros::system_param::derive_system_param_impl(input)
}
