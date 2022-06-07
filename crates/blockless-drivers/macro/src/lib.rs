mod config;
use config::BlocklessConfig;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote};
use syn::parse_macro_input;
use wiggle_generate::Names;

#[proc_macro]
pub fn linker_integration(args: TokenStream) -> proc_macro::TokenStream {
    let config = parse_macro_input!(args as BlocklessConfig);

    let doc = config.load_document();
    let names = Names::new(quote!(wiggle));
    let mut funcs = Vec::new();
    for module in doc.modules() {
        for f in module.funcs() {
            funcs.push(generate_func(&module, &f, &names, Some(&config.target)));
        }
    }
    let method_name = format_ident!("{}", config.link_method.value());
    quote!(
        pub fn #method_name(linker: &mut Linker<WasiCtx>) {
            #(#funcs)*
        }
    )
    .into()
}

fn generate_func(
    module: &witx::Module,
    func: &witx::InterfaceFunc,
    names: &Names,
    target_path: Option<&syn::Path>,
) -> proc_macro2::TokenStream {
    let module_ident = names.module(&module.name);
    let module_name = module.name.as_str();
    let rt = names.runtime_mod();
    let (params, results) = func.wasm_signature();

    let arg_names: Vec<Ident> = (0..params.len())
        .map(|i| Ident::new(&format!("arg{}", i), Span::call_site()))
        .collect::<Vec<_>>();

    let arg_decls = params
        .iter()
        .enumerate()
        .map(|(i, ty)| {
            let name = &arg_names[i];
            let wasm = names.wasm_type(*ty);
            quote! { #name: #wasm }
        })
        .collect::<Vec<_>>();

    let wrapper = format_ident!("func_wrap{}_async", params.len());
    let func_name = func.name.as_str();
    let func_ident = names.func(&func.name);
    let ret_ty = match results.len() {
        0 => quote!(()),
        1 => names.wasm_type(results[0]),
        _ => unimplemented!(),
    };
    let abi_func = quote!( #target_path::#module_ident::#func_ident );
    let linker = quote!(
        linker.#wrapper(
            #module_name,
            #func_name,
            move |mut caller: #rt::wasmtime_crate::Caller<'_, WasiCtx> #(, #arg_decls)*| {
                Box::new(async move {
                    let mem = match caller.get_export("memory") {
                        Some(#rt::wasmtime_crate::Extern::Memory(m)) => m,
                        _ => {
                            return Err(#rt::wasmtime_crate::Trap::new("missing required memory export"));
                        }
                    };
                    let (mem, ctx) = mem.data_and_store_mut(&mut caller);
                    let mem = #rt::wasmtime::WasmtimeGuestMemory::new(mem);

                    match #abi_func(ctx, &mem #(, #arg_names)*).await {
                        Ok(r) => Ok(<#ret_ty>::from(r)),
                        Err(#rt::Trap::String(err)) => Err(#rt::wasmtime_crate::Trap::new(err)),
                        Err(#rt::Trap::I32Exit(err)) => Err(#rt::wasmtime_crate::Trap::i32_exit(err)),
                    }
                })
            },
        ).unwrap();
    );
    linker.into()
}
