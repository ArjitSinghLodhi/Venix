use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, parse_macro_input};

pub fn derive_bundle_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => {
            return syn::Error::new_spanned(name, "ComponentBundle can only be derived on structs")
                .to_compile_error()
                .into();
        }
    };

    let mut types = Vec::new();
    let mut destructure_patterns = Vec::new();
    let mut tuple_bindings = Vec::new();

    match fields {
        Fields::Named(fields_named) => {
            for field in &fields_named.named {
                let f_ident = &field.ident;
                types.push(field.ty.clone());
                destructure_patterns.push(quote! { #f_ident });
                tuple_bindings.push(quote! { #f_ident });
            }
        }
        Fields::Unnamed(fields_unnamed) => {
            for (i, field) in fields_unnamed.unnamed.iter().enumerate() {
                types.push(field.ty.clone());
                let dummy_ident = Ident::new(&format!("field_{}", i), Span::call_site());
                destructure_patterns.push(quote! { #dummy_ident });
                tuple_bindings.push(quote! { #dummy_ident });
            }
        }
        Fields::Unit => {}
    }

    let field_count = types.len();

    let destructure = match fields {
        Fields::Named(_) => quote! { let Self { #(#destructure_patterns),* } = self; },
        Fields::Unnamed(_) => quote! { let Self(#(#destructure_patterns),*) = self; },
        Fields::Unit => quote! { let Self = self; },
    };

    let expanded = quote! {
        impl #impl_generics ::venix::commands::bundle::ComponentBundle for #name #ty_generics #where_clause {
            const TYPE_IDS: &'static [::std::any::TypeId] = &[
                #( ::std::any::TypeId::of::<#types>() ),*
            ];

            #[inline(always)]
            fn get_type_ids() -> &'static [::std::any::TypeId] {
                Self::TYPE_IDS
            }

            #[inline(always)]
            fn create_empty_columns(columns: &mut ::venix::indexmap::IndexMap<::std::any::TypeId, ::venix::extensions::ComponentColumn, ::venix::fxhash::FxBuildHasher>) {
                <(#(#types,)*) as ::venix::commands::bundle::ComponentBundle>::create_empty_columns(columns);
            }

            #[inline(always)]
            fn push_to_archetype(self, archetype: &mut ::venix::extensions::Archetype) {
                #destructure
                let tuple_data = (#(#tuple_bindings,)*);
                ::venix::commands::bundle::ComponentBundle::push_to_archetype(tuple_data, archetype);
            }

            type NamesArray = [&'static str; #field_count];

            #[inline(always)]
            fn get_type_names() -> Self::NamesArray {
                [
                    #( ::std::any::type_name::<#types>() ),*
                ]
            }
        }
    };

    TokenStream::from(expanded)
}
