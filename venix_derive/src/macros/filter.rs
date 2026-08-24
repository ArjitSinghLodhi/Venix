use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

pub fn derive_filter_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => {
            return syn::Error::new_spanned(name, "Filter can only be derived on structs")
                .to_compile_error()
                .into();
        }
    };

    let mut field_types = Vec::new();
    let mut field_idents = Vec::new();
    
    let is_named = matches!(fields, Fields::Named(_));
    let is_unnamed = matches!(fields, Fields::Unnamed(_));

    match fields {
        Fields::Named(fields_named) => {
            for field in &fields_named.named {
                field_types.push(field.ty.clone());
                field_idents.push(field.ident.clone().unwrap());
            }
        }
        Fields::Unnamed(fields_unnamed) => {
            for (i, field) in fields_unnamed.unnamed.iter().enumerate() {
                field_types.push(field.ty.clone());
                let dummy_ident =
                    syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site());
                field_idents.push(dummy_ident);
            }
        }
        Fields::Unit => {}
    }

    let placeholders: Vec<_> = (0..field_types.len())
        .map(|_| quote! { _ })
        .collect();

    let mock_instantiation = if is_named {
        quote! { 
            let mock_value: #name #ty_generics = unsafe { ::std::mem::zeroed() };
            let #name { #( #field_idents, )* } = mock_value;
            #( let _ = #field_idents; )*
        }
    } else if is_unnamed {
        quote! { 
            let mock_value: #name #ty_generics = unsafe { ::std::mem::zeroed() };
            let #name ( #( #placeholders, )* ) = mock_value;
        }
    } else {
        quote! {
            let _mock_value: #name #ty_generics = unsafe { ::std::mem::zeroed() };
        }
    };

    let dummy_function_name = syn::Ident::new(
        &format!("__silence_dead_code_filter_{}", name),
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

        impl #impl_generics ::venix::query::filter::Filter for #name #ty_generics #where_clause {
            #[inline(always)]
            fn matches(types: &::venix::extensions::AccessHashSet<::std::any::TypeId>) -> bool {
                <( #(#field_types,)* ) as ::venix::query::filter::Filter>::matches(types)
            }

            #[inline(always)]
            fn matches_negated(types: &::venix::extensions::AccessHashSet<::std::any::TypeId>) -> bool {
                <( #(#field_types,)* ) as ::venix::query::filter::Filter>::matches_negated(types)
            }
            #[inline(always)]
            fn collect_filter(
                withs: &mut ::venix::extensions::AccessVec<::std::any::TypeId>,
                withouts: &mut ::venix::extensions::AccessVec<::std::any::TypeId>,
            ) {
                <( #(#field_types,)* ) as ::venix::query::filter::Filter>::collect_filter(withs, withouts);
            }

            #[inline(always)]
            fn filter_indices(
                archetype: &::venix::extensions::Archetype,
                indices: &mut ::std::vec::Vec<usize>,
                system_data: &mut ::venix::extensions::FunctionData,
            ) {
                <( #(#field_types,)* ) as ::venix::query::filter::Filter>::filter_indices(archetype, indices, system_data);
            }
        }
    };

    TokenStream::from(expanded)
}
