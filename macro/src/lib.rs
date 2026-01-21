use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{
    AngleBracketedGenericArguments, Error, FnArg, GenericArgument, Ident, ItemFn, PatType,
    PathArguments, Type, TypePath, TypeReference, TypeSlice, TypeTuple, parse::Parse, parse_quote,
    spanned::Spanned,
};

#[proc_macro_attribute]
pub fn juicy(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let juicy = match syn::parse::<JuicyMain>(item) {
        Ok(juicy) => juicy,
        Err(e) => {
            let e = e.to_compile_error();
            return quote::quote!(
                fn main() {#e}
            )
            .into();
        }
    };

    juicy.to_token_stream().into()
}

#[derive(Debug)]
struct JuicyMain {
    function: ItemFn,
    args: Option<ArgsKind>,
    env: Option<EnvKind>,
    order: Order,
}

#[derive(Debug)]
enum Order {
    EnvFirst,
    ArgsFirst,
    None,
}

impl ToTokens for JuicyMain {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            function,
            args,
            env,
            order,
        } = self;

        let mut params: Vec<Ident> = vec![];

        match order {
            Order::None => {}
            Order::EnvFirst => {
                params.push(parse_quote!(env));
                if args.is_some() {
                    params.push(parse_quote!(args));
                }
            }
            Order::ArgsFirst => {
                params.push(parse_quote!(args));
                if env.is_some() {
                    params.push(parse_quote!(env));
                }
            }
        }

        quote::quote! {
            fn main() {
                #args
                #env

                #function

                main(#(#params),*)
            }
        }
        .to_tokens(tokens);
    }
}

impl Parse for JuicyMain {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let function: ItemFn = input.parse()?;

        if function.sig.ident != "main" {
            return Err(Error::new(function.sig.ident.span(), "expected main"));
        }

        let mut args = None;
        let mut env = None;
        let mut order = Order::None;

        for attr in &function.sig.inputs {
            let FnArg::Typed(PatType { ty, .. }) = attr else {
                return Err(Error::new(attr.span(), "self is not accepted"));
            };
            let ty = ty.as_ref();

            if let Ok(found) = EnvKind::try_from(ty) {
                if env.is_some() {
                    return Err(Error::new(ty.span(), "only one env input is allowed"));
                }
                if args.is_none() {
                    order = Order::EnvFirst;
                }
                env = Some(found);
            } else if let Ok(found) = ArgsKind::try_from(ty) {
                if cfg!(not(feature = "clap")) && found == ArgsKind::Parsed {
                    return Err(Error::new(
                        ty.span(),
                        "command-line parsing with clap is not enabled",
                    ));
                }
                if args.is_some() {
                    return Err(Error::new(ty.span(), "only one args input is allowed"));
                }
                if env.is_none() {
                    order = Order::ArgsFirst;
                }
                args = Some(found);
            } else {
                return Err(Error::new(ty.span(), "invalid input"));
            }
        }

        Ok(Self {
            function,
            args,
            env,
            order,
        })
    }
}

fn generic_is_string(arg: &GenericArgument) -> bool {
    if let GenericArgument::Type(ty) = arg {
        type_is_string(ty)
    } else {
        false
    }
}

fn type_is_string(value: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = value {
        path.segments
            .last()
            .is_some_and(|seg| seg.ident == "String")
    } else {
        false
    }
}

#[derive(Debug)]
enum EnvKind {
    Slice,
    Vec,
    Iterator,
    HashMap,
}

impl TryFrom<&Type> for EnvKind {
    type Error = Error;

    fn try_from(value: &Type) -> Result<Self, Self::Error> {
        let expected_one_of = || {
            Err(Error::new(
                value.span(),
                "expected one of &[(String, String)], Vars, Vec<(String, String)>, or HashMap<String, String>",
            ))
        };

        fn type_is_var(ty: &Type) -> bool {
            if let Type::Tuple(TypeTuple { elems, .. }) = ty
                && elems.len() == 2
                && type_is_string(&elems[0])
                && type_is_string(&elems[1])
            {
                true
            } else {
                false
            }
        }

        match value {
            Type::Reference(TypeReference { elem, .. })
                if {
                    if let Type::Slice(TypeSlice { elem, .. }) = elem.as_ref()
                        && type_is_var(elem)
                    {
                        true
                    } else {
                        false
                    }
                } =>
            {
                Ok(EnvKind::Slice)
            }
            Type::Path(TypePath { path, .. }) => {
                let seg = path.segments.last().expect("empty path");

                if seg.ident == "Vars" {
                    return Ok(EnvKind::Iterator);
                }

                let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
                    &seg.arguments
                else {
                    return expected_one_of();
                };

                match seg.ident.to_string().as_str() {
                    "Vec"
                        if args.len() == 1 && {
                            if let GenericArgument::Type(ty) = &args[0] {
                                type_is_var(ty)
                            } else {
                                false
                            }
                        } =>
                    {
                        Ok(EnvKind::Vec)
                    }
                    "HashMap"
                        if args.len() == 2
                            && generic_is_string(&args[0])
                            && generic_is_string(&args[1]) =>
                    {
                        Ok(EnvKind::HashMap)
                    }
                    _ => expected_one_of(),
                }
            }
            _ => expected_one_of(),
        }
    }
}

impl ToTokens for EnvKind {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Slice => quote::quote! {
                let vec = ::std::env::vars().collect::<::std::vec::Vec<_>>();
                let env = vec.as_slice();
            },
            Self::Vec => quote::quote!(let env = ::std::env::vars().collect::<::std::vec::Vec<_>>();),
            Self::Iterator => quote::quote!(let env = ::std::env::vars();),
            Self::HashMap => quote::quote!(let env = ::std::env::vars().collect::<::std::collections::HashMap<_, _>>();),
        }
        .to_tokens(tokens);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ArgsKind {
    Slice,
    Vec,
    Iterator,
    Parsed,
}

impl TryFrom<&Type> for ArgsKind {
    type Error = Error;

    fn try_from(value: &Type) -> Result<Self, Self::Error> {
        match value {
            Type::Reference(TypeReference { elem, .. })
                if {
                    if let Type::Slice(TypeSlice { elem, .. }) = elem.as_ref()
                        && type_is_string(elem)
                    {
                        true
                    } else {
                        false
                    }
                } =>
            {
                Ok(ArgsKind::Slice)
            }
            Type::Path(TypePath { path, .. }) => {
                let seg = path.segments.last().expect("empty path");
                if seg.ident == "Args" {
                    return Ok(ArgsKind::Iterator);
                }
                let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
                    &seg.arguments
                else {
                    return Ok(ArgsKind::Parsed);
                };

                if seg.ident == "Vec" && args.len() == 1 && generic_is_string(&args[0]) {
                    Ok(ArgsKind::Vec)
                } else {
                    Ok(ArgsKind::Parsed)
                }
            }
            _ => Ok(ArgsKind::Parsed),
        }
    }
}

impl ToTokens for ArgsKind {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Slice => quote::quote! {
                let vec = ::std::env::args().collect::<::std::vec::Vec<_>>();
                let args = vec.as_slice();
            },
            Self::Vec => {
                quote::quote!(let args = ::std::env::args().collect::<::std::vec::Vec<_>>();)
            }
            Self::Iterator => quote::quote!(let args = ::std::env::args();),
            Self::Parsed => quote::quote!(let args = ::juicy_main::clap::Parser::parse();),
        }
        .to_tokens(tokens);
    }
}
