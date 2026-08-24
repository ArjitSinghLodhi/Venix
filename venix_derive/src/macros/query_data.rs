use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Index, parse_macro_input};

pub fn derive_query_data_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let visibility = &input.vis; 
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => {
            return syn::Error::new_spanned(name, "WorldQuery can only be derived on structs")
                .to_compile_error()
                .into();
        }
    };

    let mut field_types = Vec::new();
    let mut field_idents = Vec::new();
    let mut field_visibilities = Vec::new();
    let mut tuple_indices = Vec::new();
    
    let is_named = matches!(fields, Fields::Named(_));

    match fields {
        Fields::Named(fields_named) => {
            for (i, field) in fields_named.named.iter().enumerate() {
                field_types.push(field.ty.clone());
                field_idents.push(field.ident.clone().unwrap());
                field_visibilities.push(field.vis.clone());
                tuple_indices.push(Index::from(i));
            }
        }
        Fields::Unnamed(fields_unnamed) => {
            for (i, field) in fields_unnamed.unnamed.iter().enumerate() {
                field_types.push(field.ty.clone());
                let dummy_ident =
                    syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site());
                field_idents.push(dummy_ident);
                field_visibilities.push(field.vis.clone());
                tuple_indices.push(Index::from(i));
            }
        }
        Fields::Unit => {}
    }

    let item_struct_name = syn::Ident::new(&format!("{}Item", name), proc_macro2::Span::call_site());
    let readonly_item_name = syn::Ident::new(&format!("{}ReadOnlyItem", name), proc_macro2::Span::call_site());

    let item_fields = if is_named {
        quote! {
            #( #field_visibilities #field_idents: <#field_types as ::venix::query::params::WorldQuery>::Item<'w>, )*
        }
    } else {
        quote! {
            #( #field_visibilities <#field_types as ::venix::query::params::WorldQuery>::Item<'w>, )*
        }
    };

    let readonly_item_fields = if is_named {
        quote! {
            #( #field_visibilities #field_idents: <#field_types as ::venix::query::params::WorldQuery>::ReadOnlyItem<'w>, )*
        }
    } else {
        quote! {
            #( #field_visibilities <#field_types as ::venix::query::params::WorldQuery>::ReadOnlyItem<'w>, )*
        }
    };

    let item_struct_def = if is_named {
        quote! { #visibility struct #item_struct_name<'w> { #item_fields } }
    } else {
        quote! { #visibility struct #item_struct_name<'w> ( #item_fields ); }
    };

    let readonly_item_def = if is_named {
        quote! { #visibility struct #readonly_item_name<'w> { #readonly_item_fields } }
    } else {
        quote! { #visibility struct #readonly_item_name<'w> ( #readonly_item_fields ); }
    };

    let item_construction = if is_named {
        quote! { #item_struct_name { #( #field_idents: tuple_res.#tuple_indices ),* } }
    } else {
        quote! { #item_struct_name ( #( tuple_res.#tuple_indices ),* ) }
    };

    let readonly_item_construction = if is_named {
        quote! { #readonly_item_name { #( #field_idents: tuple_res.#tuple_indices ),* } }
    } else {
        quote! { #readonly_item_name ( #( tuple_res.#tuple_indices ),* ) }
    };

    let placeholders: Vec<_> = (0..field_types.len())
        .map(|_| quote! { _ })
        .collect();

    let mock_instantiation = if is_named {
        quote! { 
            let mock_value: #name #ty_generics = unsafe { ::std::mem::zeroed() };
            let #name { #( #field_idents, )* } = mock_value;
            #( let _ = #field_idents; )*
        }
    } else {
        quote! { 
            let mock_value: #name #ty_generics = unsafe { ::std::mem::zeroed() };
            let #name ( #( #placeholders, )* ) = mock_value;
        }
    };

    let dummy_function_name = syn::Ident::new(
        &format!("__silence_dead_code_{}", name),
        proc_macro2::Span::call_site(),
    );

    let expanded = quote! {
        #[allow(dead_code)]
        const _: () = {
            #[inline(always)]
            fn #dummy_function_name #impl_generics () #where_clause {
                #mock_instantiation
            }
        };

        #[allow(dead_code)]
        #item_struct_def

        #[allow(dead_code)]
        #readonly_item_def

        impl #impl_generics ::venix::query::params::WorldQuery for #name #ty_generics #where_clause {
            type Item<'w> = #item_struct_name<'w>;
            type ReadOnlyItem<'w> = #readonly_item_name<'w>;
            type Fetch = <( #(#field_types,)* ) as ::venix::query::params::WorldQuery>::Fetch;

            #[inline(always)]
            fn matches(types: &::venix::fxhash::FxHashSet<::std::any::TypeId>) -> bool {
                <( #(#field_types,)* ) as ::venix::query::params::WorldQuery>::matches(types)
            }
            #[inline(always)]
            unsafe fn init_fetch(
                archetype: &::venix::extensions::Archetype,
                systems_data: &mut ::venix::extensions::FunctionData,
            ) -> Self::Fetch {
                unsafe {
                    <( #(#field_types,)* ) as ::venix::query::params::WorldQuery>::init_fetch(
                        archetype,
                        systems_data,
                    )
                }
            }
            #[inline(always)]
            fn collect_access(
                reads: &mut ::venix::extensions::AccessVec<::std::any::TypeId>,
                writes: &mut ::venix::extensions::AccessVec<::std::any::TypeId>,
            ) {
                <( #(#field_types,)* ) as ::venix::query::params::WorldQuery>::collect_access(
                    reads,
                    writes,
                );
            }
            #[inline(always)]
            unsafe fn fetch_mut<'w>(fetch: Self::Fetch, index: usize) -> Self::Item<'w> {
                let tuple_res = unsafe {
                    <( #(#field_types,)* ) as ::venix::query::params::WorldQuery>::fetch_mut(
                        fetch,
                        index,
                    )
                };
                #item_construction
            }
            #[inline(always)]
            unsafe fn fetch_read_only<'w>(fetch: Self::Fetch, index: usize) -> Self::ReadOnlyItem<'w> {
                let tuple_res = unsafe {
                    <( #(#field_types,)* ) as ::venix::query::params::WorldQuery>::fetch_read_only(
                        fetch,
                        index,
                    )
                };
                #readonly_item_construction
            }
        }
    };
    TokenStream::from(expanded)
}
