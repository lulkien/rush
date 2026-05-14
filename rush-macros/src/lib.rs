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
                plugin_name: rush_internal_plugin_name,
                plugin_desc: rush_internal_plugin_desc,
                plugin_help: rush_internal_plugin_help,
                plugin_version: rush_internal_plugin_version,
                execute: rush_internal_execute,
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
pub fn plugin_name(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &function.sig.ident;

    quote! {
        #[::abi_stable::sabi_extern_fn]
        fn rush_internal_plugin_name() -> ::abi_stable::std_types::RString {
            #function

            #fn_name()
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn plugin_desc(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &function.sig.ident;

    quote! {
        #[::abi_stable::sabi_extern_fn]
        fn rush_internal_plugin_desc() -> ::abi_stable::std_types::RString {
            #function

            #fn_name()
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn plugin_help(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &function.sig.ident;

    quote! {
        #[::abi_stable::sabi_extern_fn]
        fn rush_internal_plugin_help() -> ::abi_stable::std_types::RString {
            #function

            #fn_name()
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn plugin_version(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &function.sig.ident;

    quote! {
        #[::abi_stable::sabi_extern_fn]
        fn rush_internal_plugin_version() -> ::abi_stable::std_types::RString {
            #function

            #fn_name()
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn execute(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &function.sig.ident;

    quote! {
        #[::abi_stable::sabi_extern_fn]
        fn rush_internal_execute(
            args: ::abi_stable::std_types::RVec<::abi_stable::std_types::RString>) -> ::rush_plugin::rush_interface::ExecResult {
            #function

            #fn_name(args)
        }
    }
    .into()
}
