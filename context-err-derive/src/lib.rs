use darling::FromMeta;
use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{parse_macro_input, spanned::Spanned, AttributeArgs, Item, ItemEnum, ItemStruct};

#[derive(Debug, FromMeta)]
struct Args {
    #[darling(rename = "trait")]
    trait_: Option<String>,
}

#[proc_macro_attribute]
pub fn derive_context_err(args: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args = parse_macro_input!(args as AttributeArgs);
    let item = parse_macro_input!(item as Item);

    let args = match Args::from_list(&attr_args) {
        Ok(args) => args,
        Err(err) => return TokenStream::from(err.write_errors()),
    };

    match item {
        Item::Enum(item) => derive_for_enum(args, item),
        Item::Struct(item) => derive_for_struct(args, item),
        _ => quote_spanned! {
            item.span() => compile_error!("this macro only works for structs and enums")
        }
        .into(),
    }
}

fn derive_for_enum(args: Args, item: ItemEnum) -> TokenStream {
    todo!()
}

fn derive_for_struct(args: Args, item: ItemStruct) -> TokenStream {
    todo!()
}
