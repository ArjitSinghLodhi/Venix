use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

pub fn derive_system_param_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => {
            return syn::Error::new_spanned(name, "SystemParam can only be derived on structs")
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

    let extract_fields = if is_named {
        quote! {
            #name {
                #( #field_idents: <#field_types as ::venix::extensions::SystemParam>::extract(world, system_data) ),*
            }
        }
    } else if is_unnamed {
        quote! {
            #name(
                #( <#field_types as ::venix::extensions::SystemParam>::extract(world, system_data) ),*
            )
        }
    } else {
        quote! { #name }
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
        &format!("__silence_dead_code_sysparam_{}", name),
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

        impl #impl_generics ::venix::extensions::SystemParam for #name #ty_generics #where_clause {
            #[inline(always)]
            fn get_access() -> ::venix::extensions::ParamAccess {
                let mut access = ::venix::extensions::ParamAccess::default();

                #(
                    let field_access = <#field_types as ::venix::extensions::SystemParam>::get_access();
                    for &id in field_access.reads.iter() { access.reads.push(id); }
                    for &id in field_access.writes.iter() { access.writes.push(id); }
                    for &id in field_access.with_filters.iter() { access.with_filters.push(id); }
                    for &id in field_access.without_filters.iter() { access.without_filters.push(id); }
                    for &id in field_access.res_reads.iter() { access.res_reads.push(id); }
                    for &id in field_access.res_writes.iter() { access.res_writes.push(id); }
                )*

                access
            }

            #[inline(always)]
            fn extract(
                world: &mut ::venix::prelude::World,
                system_data: &mut ::venix::extensions::FunctionData,
            ) -> Self {
                #extract_fields
            }
        }
    };

    TokenStream::from(expanded)
}
