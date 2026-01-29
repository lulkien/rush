use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;


#[proc_macro_attribute]
pub fn load(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &function.sig.ident;

    quote! {
        #[::abi_stable::export_root_module]
        fn ffi_internal_init_root_module() -> ::rush_plugin::rush_interface::CommandRef {
            use ::abi_stable::prefix_type::PrefixTypeTrait;

            ::rush_plugin::rush_interface::Command {
                load: rush_internal_load,
                info: rush_internal_info,
                version: rush_internal_version,
                usage: rush_internal_usage,
                exec: rush_internal_exec,
            }
            .leak_into_prefix()
        }

        #[::abi_stable::sabi_extern_fn]
        fn rush_internal_load() {
            #function

            #fn_name()
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn info(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &function.sig.ident;

    quote! {
        #[::abi_stable::sabi_extern_fn]
        fn rush_internal_info() -> ::rush_plugin::rush_interface::CommandInfo {
            #function

            #fn_name()
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn version(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &function.sig.ident;

    quote! {
        #[::abi_stable::sabi_extern_fn]
        fn rush_internal_version() -> ::abi_stable::std_types::RString {
            #function

            #fn_name()
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn usage(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &function.sig.ident;

    quote! {
        #[::abi_stable::sabi_extern_fn]
        fn rush_internal_usage() -> ::abi_stable::std_types::RString {
            #function

            #fn_name()
        }
    }
    .into()
}


#[proc_macro_attribute]
pub fn exec(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &function.sig.ident;

    quote! {
        #[::abi_stable::sabi_extern_fn]
        fn rush_internal_exec(args: ::abi_stable::std_types::RVec<::abi_stable::std_types::RString>)
            -> ::rush_plugin::rush_interface::ExecResult {
            #function
            
            #fn_name(args)
        }
    }
    .into()
}
