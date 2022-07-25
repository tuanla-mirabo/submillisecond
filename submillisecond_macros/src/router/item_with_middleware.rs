use proc_macro2::TokenStream;
use syn::{
    braced, custom_keyword,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token, Ident, Token,
};

use crate::hquote;

custom_keyword!(with);

#[derive(Clone, Debug)]
pub struct ItemUseMiddleware {
    pub use_token: with,
    pub leading_colon: Option<Token![::]>,
    pub tree: UseMiddlewareTree,
}

impl Parse for ItemUseMiddleware {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ItemUseMiddleware {
            use_token: input.parse()?,
            leading_colon: input.parse()?,
            tree: input.parse()?,
        })
    }
}

#[derive(Clone, Debug)]
pub enum UseMiddlewareTree {
    Path(UseMiddlewarePath),
    Name(UseMiddlewareName),
    Group(UseMiddlewareGroup),
}

impl UseMiddlewareTree {
    pub fn items(&self) -> Vec<TokenStream> {
        match self {
            UseMiddlewareTree::Path(UseMiddlewarePath { ident, tree, .. }) => tree
                .items()
                .into_iter()
                .map(|item| hquote! { #ident::#item })
                .collect(),
            UseMiddlewareTree::Name(UseMiddlewareName { ident }) => {
                vec![hquote! { #ident }]
            }
            UseMiddlewareTree::Group(UseMiddlewareGroup { items, .. }) => {
                items.iter().flat_map(UseMiddlewareTree::items).collect()
            }
        }
    }
}

impl Parse for UseMiddlewareTree {
    #[allow(clippy::eval_order_dependence)]
    fn parse(input: ParseStream) -> syn::Result<UseMiddlewareTree> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Ident)
            || lookahead.peek(Token![self])
            || lookahead.peek(Token![super])
            || lookahead.peek(Token![crate])
        {
            let ident = input.call(Ident::parse_any)?;
            if input.peek(Token![::]) {
                Ok(UseMiddlewareTree::Path(UseMiddlewarePath {
                    ident,
                    colon2_token: input.parse()?,
                    tree: Box::new(input.parse()?),
                }))
            } else if input.peek(Token![as]) {
                Err(input.error("use as is not supported"))
            } else {
                Ok(UseMiddlewareTree::Name(UseMiddlewareName { ident }))
            }
        } else if lookahead.peek(Token![*]) {
            Err(input.error("use * is not supported"))
        } else if lookahead.peek(token::Brace) {
            let content;
            Ok(UseMiddlewareTree::Group(UseMiddlewareGroup {
                brace_token: braced!(content in input),
                items: content.parse_terminated(UseMiddlewareTree::parse)?,
            }))
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Clone, Debug)]
pub struct UseMiddlewarePath {
    pub ident: Ident,
    pub colon2_token: Token![::],
    pub tree: Box<UseMiddlewareTree>,
}

#[derive(Clone, Debug)]
pub struct UseMiddlewareName {
    pub ident: Ident,
}

#[derive(Clone, Debug)]
pub struct UseMiddlewareGroup {
    pub brace_token: token::Brace,
    pub items: Punctuated<UseMiddlewareTree, Token![,]>,
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::ItemUseMiddleware;

    #[test]
    fn item_with_items() {
        let item_use: ItemUseMiddleware = parse_quote! {
            with ::a::b::c::{logger, foo}
        };
        let items = item_use.tree.items();
        assert_eq!(
            items
                .iter()
                .map(|list| list.to_string().replace(' ', ""))
                .collect::<Vec<_>>(),
            ["a::b::c::logger", "a::b::c::foo"]
        );
    }
}
