use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    ItemFn, LitInt, LitStr, Meta, Token,
};

/// Rename local bindings. Unsupported syntax passes through unless `strict` is set.
#[proc_macro_attribute]
pub fn obfuscate(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with Punctuated::<Meta, Token![,]>::parse_terminated);
    let mut seed = 0u64;
    let mut strict = false;
    let mut assume_bindings = false;
    for arg in args {
        match arg {
            Meta::Path(p) if p.is_ident("strict") => strict = true,
            Meta::Path(p) if p.is_ident("assume_bindings") => assume_bindings = true,
            Meta::NameValue(m) if m.path.is_ident("seed") => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(n),
                    ..
                }) = &m.value
                {
                    match n.base10_parse::<u64>() {
                        Ok(n) => seed = n,
                        Err(e) => return e.to_compile_error().into(),
                    }
                } else {
                    return syn::Error::new_spanned(m, "seed must be a u64 integer literal")
                        .to_compile_error()
                        .into();
                }
            }
            other => {
                return syn::Error::new_spanned(
                    other,
                    "expected seed = <u64>, strict, or assume_bindings",
                )
                .to_compile_error()
                .into()
            }
        }
    }
    if !cfg!(feature = "enabled") {
        return input;
    }
    let mut function = parse_macro_input!(input as ItemFn);
    let result = if function.attrs.is_empty() {
        rustveil_core::transform_with_options(
            &function.sig,
            &mut function.block,
            seed,
            assume_bindings,
        )
    } else {
        rustveil_core::Outcome { renamed: 0, skipped: Some("other function attributes require explicit review; use an unannotated helper function".into()) }
    };
    if strict {
        if let Some(reason) = result.skipped {
            return syn::Error::new_spanned(
                &function.sig,
                format!("rustveil skipped function: {reason}"),
            )
            .to_compile_error()
            .into();
        }
    }
    quote!(#function).into()
}

struct StringInput {
    value: LitStr,
    seed: u64,
}
impl Parse for StringInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let value = input.parse()?;
        let mut seed = 0;
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if !input.is_empty() {
                seed = input.parse::<LitInt>()?.base10_parse()?;
            }
        }
        if !input.is_empty() {
            return Err(input.error("expected one string literal and an optional u64 seed"));
        }
        Ok(Self { value, seed })
    }
}

/// Produce a cached &'static str. This is runtime obfuscation, not encryption.
#[proc_macro]
pub fn veil_str(input: TokenStream) -> TokenStream {
    let StringInput { value, seed } = parse_macro_input!(input as StringInput);
    if !cfg!(feature = "enabled") {
        return quote!(#value).into();
    }
    let plaintext = value.value();
    let seed = rustveil_core::hash(plaintext.as_bytes(), seed);
    let mut state = seed;
    let bytes: Vec<u8> = plaintext
        .bytes()
        .map(|b| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            b ^ (state >> 56) as u8
        })
        .collect();
    let runtime = match proc_macro_crate::crate_name("rustveil") {
        Ok(proc_macro_crate::FoundCrate::Itself) => quote!(::rustveil),
        Ok(proc_macro_crate::FoundCrate::Name(name)) => {
            let id = proc_macro2::Ident::new(&name, proc_macro2::Span::call_site());
            quote!(::#id)
        }
        Err(e) => {
            return syn::Error::new(
                value.span(),
                format!("add the rustveil facade as a dependency: {e}"),
            )
            .to_compile_error()
            .into()
        }
    };
    quote!({
        static VALUE: ::std::sync::OnceLock<::std::string::String> = ::std::sync::OnceLock::new();
        VALUE.get_or_init(|| #runtime::__private::decode(&[#(#bytes),*], #seed)).as_str()
    })
    .into()
}
