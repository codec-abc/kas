// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

use std::collections::HashMap;

use proc_macro2::{Punct, Spacing, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Brace, Colon, Comma, Eq, For, Impl, Paren};
use syn::{braced, bracketed, parenthesized, parse_quote};
use syn::{
    Attribute, ConstParam, Data, DeriveInput, Expr, Fields, FieldsNamed, FieldsUnnamed,
    GenericParam, Generics, Ident, Index, Lifetime, LifetimeDef, Lit, Member, Token, Type,
    TypeParam, TypePath, TypeTraitObject,
};

#[derive(Debug)]
pub struct Child {
    pub ident: Member,
    pub args: WidgetAttrArgs,
}

pub struct Args {
    pub core_data: Member,
    pub layout_data: Option<Member>,
    pub widget: WidgetArgs,
    pub layout: Option<LayoutArgs>,
    pub handler: Vec<HandlerArgs>,
    pub children: Vec<Child>,
}

pub fn read_attrs(ast: &mut DeriveInput) -> Result<Args> {
    let not_struct_err = |span| {
        Err(Error::new(
            span,
            "cannot derive Widget on an enum, union or unit struct",
        ))
    };
    let (fields, span) = match &mut ast.data {
        Data::Struct(data) => match &mut data.fields {
            Fields::Named(FieldsNamed {
                brace_token: Brace { span },
                named: fields,
            })
            | Fields::Unnamed(FieldsUnnamed {
                paren_token: Paren { span },
                unnamed: fields,
            }) => (fields, span),
            Fields::Unit => return not_struct_err(data.struct_token.span()),
        },
        Data::Enum(data) => return not_struct_err(data.enum_token.span()),
        Data::Union(data) => return not_struct_err(data.union_token.span()),
    };

    let mut core_data = None;
    let mut layout_data = None;
    let mut children = vec![];

    for (i, field) in fields.iter_mut().enumerate() {
        for attr in field.attrs.drain(..) {
            if attr.path == parse_quote! { layout } || attr.path == parse_quote! { handler } {
                // These are valid attributes according to proc_macro_derive, so we need to catch them
                #[cfg(nightly)]
                attr.span()
                    .unwrap()
                    .error("invalid attribute on Widget field (applicable to struct only)")
                    .emit()
            } else if attr.path == parse_quote! { widget_core } {
                if core_data.is_none() {
                    core_data = Some(member(i, field.ident.clone()));
                } else {
                    #[cfg(nightly)]
                    attr.span()
                        .unwrap()
                        .error("multiple fields marked with #[widget_core]")
                        .emit();
                }
            } else if attr.path == parse_quote! { layout_data } {
                if layout_data.is_none() {
                    if field.ty != parse_quote! { <Self as kas::LayoutData>::Data }
                        && field.ty != parse_quote! { <Self as LayoutData>::Data }
                    {
                        #[cfg(nightly)]
                        field
                            .ty
                            .span()
                            .unwrap()
                            .warning("expected type `<Self as kas::LayoutData>::Data`")
                            .emit();
                    }
                    layout_data = Some(member(i, field.ident.clone()));
                } else {
                    #[cfg(nightly)]
                    attr.span()
                        .unwrap()
                        .error("multiple fields marked with #[layout_data]")
                        .emit();
                }
            } else if attr.path == parse_quote! { widget } {
                let ident = member(i, field.ident.clone());
                let args = syn::parse2(attr.tokens)?;
                children.push(Child { ident, args });
            }
        }
    }

    let mut widget = None;
    let mut layout = None;
    let mut handler = vec![];

    for attr in ast.attrs.drain(..) {
        if attr.path == parse_quote! { widget_core } || attr.path == parse_quote! { layout_data } {
            // These are valid attributes according to proc_macro_derive, so we need to catch them
            #[cfg(nightly)]
            attr.span()
                .unwrap()
                .error("invalid attribute on Widget struct (applicable to fields only)")
                .emit()
        } else if attr.path == parse_quote! { widget } {
            if widget.is_none() {
                widget = Some(syn::parse2(attr.tokens)?);
            } else {
                #[cfg(nightly)]
                attr.span()
                    .unwrap()
                    .error("multiple #[widget(..)] attributes on type")
                    .emit()
            }
        } else if attr.path == parse_quote! { layout } {
            if layout.is_none() {
                layout = Some(syn::parse2(attr.tokens)?);
            } else {
                #[cfg(nightly)]
                attr.span()
                    .unwrap()
                    .error("multiple #[layout(..)] attributes on type")
                    .emit()
            }
        } else if attr.path == parse_quote! { handler } {
            handler.push(syn::parse2(attr.tokens)?);
        }
    }

    let widget = widget.unwrap_or(WidgetArgs::default());

    if let Some(core_data) = core_data {
        Ok(Args {
            core_data,
            layout_data,
            widget,
            layout,
            handler,
            children,
        })
    } else {
        Err(Error::new(
            *span,
            "one field must be marked with #[widget_core] when deriving Widget",
        ))
    }
}

fn member(index: usize, ident: Option<Ident>) -> Member {
    match ident {
        None => Member::Unnamed(Index {
            index: index as u32,
            span: Span::call_site(),
        }),
        Some(ident) => Member::Named(ident),
    }
}

#[allow(non_camel_case_types)]
mod kw {
    use syn::custom_keyword;

    custom_keyword!(area);
    custom_keyword!(layout);
    custom_keyword!(col);
    custom_keyword!(row);
    custom_keyword!(cspan);
    custom_keyword!(rspan);
    custom_keyword!(widget);
    custom_keyword!(handler);
    custom_keyword!(msg);
    custom_keyword!(generics);
    custom_keyword!(single);
    custom_keyword!(right);
    custom_keyword!(left);
    custom_keyword!(down);
    custom_keyword!(up);
    custom_keyword!(grid);
    custom_keyword!(halign);
    custom_keyword!(valign);
    custom_keyword!(key_nav);
    custom_keyword!(cursor_icon);
    custom_keyword!(handle);
    custom_keyword!(send);
    custom_keyword!(config);
    custom_keyword!(noauto);
    custom_keyword!(children);
    custom_keyword!(column);
}

#[derive(Debug)]
pub struct WidgetAttrArgs {
    pub col: Option<Lit>,
    pub row: Option<Lit>,
    pub cspan: Option<Lit>,
    pub rspan: Option<Lit>,
    pub halign: Option<Ident>,
    pub valign: Option<Ident>,
    pub handler: Option<Ident>,
}

#[derive(Debug)]
pub struct GridPos(pub u32, pub u32, pub u32, pub u32);

impl WidgetAttrArgs {
    // Parse widget position, filling in missing information with defaults.
    pub fn as_pos(&self) -> Result<GridPos> {
        fn parse_lit(lit: &Lit) -> Result<u32> {
            match lit {
                Lit::Int(li) => li.base10_parse(),
                _ => Err(Error::new(lit.span(), "expected integer literal")),
            }
        }

        Ok(GridPos(
            self.col.as_ref().map(parse_lit).unwrap_or(Ok(0))?,
            self.row.as_ref().map(parse_lit).unwrap_or(Ok(0))?,
            self.cspan.as_ref().map(parse_lit).unwrap_or(Ok(1))?,
            self.rspan.as_ref().map(parse_lit).unwrap_or(Ok(1))?,
        ))
    }

    fn match_align(ident: &Ident, horiz: bool) -> Result<TokenStream> {
        Ok(match ident {
            ident if ident == "default" => quote! { kas::Align::Default },
            ident if horiz && ident == "left" => quote! { kas::Align::TL },
            ident if !horiz && ident == "top" => quote! { kas::Align::TL },
            ident if ident == "centre" || ident == "center" => quote! { kas::Align::Centre },
            ident if horiz && ident == "right" => quote! { kas::Align::BR },
            ident if !horiz && ident == "bottom" => quote! { kas::Align::BR },
            ident if ident == "stretch" => quote! { kas::Align::Stretch },
            ident => {
                return Err(Error::new(
                    ident.span(),
                    "expected one of `default`, `centre`, `stretch`, `top` or `bottom` (if vertical), `left` or `right` (if horizontal)",
                ));
            }
        })
    }
    pub fn halign_toks(&self) -> Result<Option<TokenStream>> {
        if let Some(ref ident) = self.halign {
            Ok(Some(Self::match_align(ident, true)?))
        } else {
            Ok(None)
        }
    }
    pub fn valign_toks(&self) -> Result<Option<TokenStream>> {
        if let Some(ref ident) = self.valign {
            Ok(Some(Self::match_align(ident, false)?))
        } else {
            Ok(None)
        }
    }
}

impl Parse for WidgetAttrArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = WidgetAttrArgs {
            col: None,
            row: None,
            cspan: None,
            rspan: None,
            halign: None,
            valign: None,
            handler: None,
        };
        if input.is_empty() {
            return Ok(args);
        }

        let content;
        let _ = parenthesized!(content in input);

        loop {
            let lookahead = content.lookahead1();
            if args.col.is_none() && lookahead.peek(kw::col) {
                let _: kw::col = content.parse()?;
                let _: Eq = content.parse()?;
                args.col = Some(content.parse()?);
            } else if args.col.is_none() && lookahead.peek(kw::column) {
                let _: kw::column = content.parse()?;
                let _: Eq = content.parse()?;
                args.col = Some(content.parse()?);
            } else if args.row.is_none() && lookahead.peek(kw::row) {
                let _: kw::row = content.parse()?;
                let _: Eq = content.parse()?;
                args.row = Some(content.parse()?);
            } else if args.cspan.is_none() && lookahead.peek(kw::cspan) {
                let _: kw::cspan = content.parse()?;
                let _: Eq = content.parse()?;
                args.cspan = Some(content.parse()?);
            } else if args.rspan.is_none() && lookahead.peek(kw::rspan) {
                let _: kw::rspan = content.parse()?;
                let _: Eq = content.parse()?;
                args.rspan = Some(content.parse()?);
            } else if args.halign.is_none() && lookahead.peek(kw::halign) {
                let _: kw::halign = content.parse()?;
                let _: Eq = content.parse()?;
                args.halign = Some(content.parse()?);
            } else if args.valign.is_none() && lookahead.peek(kw::valign) {
                let _: kw::valign = content.parse()?;
                let _: Eq = content.parse()?;
                args.valign = Some(content.parse()?);
            } else if args.handler.is_none() && lookahead.peek(kw::handler) {
                let _: kw::handler = content.parse()?;
                let _: Eq = content.parse()?;
                args.handler = Some(content.parse()?);
            } else {
                return Err(lookahead.error());
            }

            if content.is_empty() {
                break;
            }
            let _: Comma = content.parse()?;
        }

        Ok(args)
    }
}

impl ToTokens for WidgetAttrArgs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if self.col.is_some()
            || self.row.is_some()
            || self.cspan.is_some()
            || self.rspan.is_some()
            || self.halign.is_some()
            || self.valign.is_some()
            || self.handler.is_some()
        {
            let comma = TokenTree::from(Punct::new(',', Spacing::Alone));
            let mut args = TokenStream::new();
            if let Some(ref lit) = self.col {
                args.append_all(quote! { col = #lit });
            }
            if let Some(ref lit) = self.row {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote! { row = #lit });
            }
            if let Some(ref lit) = self.cspan {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote! { cspan = #lit });
            }
            if let Some(ref lit) = self.rspan {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote! { rspan = #lit });
            }
            if let Some(ref ident) = self.halign {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote! { halign = #ident });
            }
            if let Some(ref ident) = self.valign {
                if !args.is_empty() {
                    args.append(comma.clone());
                }
                args.append_all(quote! { valign = #ident });
            }
            if let Some(ref ident) = self.handler {
                if !args.is_empty() {
                    args.append(comma);
                }
                args.append_all(quote! { handler = #ident });
            }
            tokens.append_all(quote! { ( #args ) });
        }
    }
}

#[derive(Debug)]
pub struct WidgetAttr {
    pub args: WidgetAttrArgs,
}

impl ToTokens for WidgetAttr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let args = &self.args;
        tokens.append_all(quote! { #[widget #args] });
    }
}

impl ToTokens for GridPos {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (c, r, cs, rs) = (&self.0, &self.1, &self.2, &self.3);
        tokens.append_all(quote! { (#c, #r, #cs, #rs) });
    }
}

pub struct WidgetArgs {
    pub config: Option<WidgetConfig>,
    pub children: bool,
}

impl Default for WidgetArgs {
    fn default() -> Self {
        WidgetArgs {
            config: Some(WidgetConfig::default()),
            children: true,
        }
    }
}

pub struct WidgetConfig {
    pub key_nav: bool,
    pub cursor_icon: Expr,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        WidgetConfig {
            key_nav: false,
            cursor_icon: parse_quote! { kas::event::CursorIcon::Default },
        }
    }
}

impl Parse for WidgetArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut config = None;
        let mut have_config = false;
        let mut children = true;
        let mut have_children = false;

        if !input.is_empty() {
            let content;
            let _ = parenthesized!(content in input);

            while !content.is_empty() {
                let lookahead = content.lookahead1();
                if lookahead.peek(kw::children) && !have_children {
                    have_children = true;
                    let _: kw::children = content.parse()?;
                    let _: Eq = content.parse()?;
                    let _: kw::noauto = content.parse()?;
                    children = false;
                } else if lookahead.peek(kw::config) && !have_config {
                    have_config = true;
                    let _: kw::config = content.parse()?;

                    if content.peek(Eq) {
                        let _: Eq = content.parse()?;
                        let lookahead = content.lookahead1();
                        if lookahead.peek(kw::noauto) {
                            let _: kw::noauto = content.parse()?;
                        } else {
                            return Err(lookahead.error());
                        }
                    } else if content.peek(syn::token::Paren) {
                        let content2;
                        let _ = parenthesized!(content2 in content);

                        let mut conf = WidgetConfig::default();
                        let mut have_key_nav = false;
                        let mut have_cursor_icon = false;

                        while !content2.is_empty() {
                            let lookahead = content2.lookahead1();
                            if lookahead.peek(kw::noauto) && !have_key_nav && !have_cursor_icon {
                                let _: kw::noauto = content2.parse()?;
                                break;
                            } else if lookahead.peek(kw::key_nav) && !have_key_nav {
                                let _: kw::key_nav = content2.parse()?;
                                let _: Eq = content2.parse()?;
                                let value: syn::LitBool = content2.parse()?;
                                conf.key_nav = value.value;
                                have_key_nav = true;
                            } else if lookahead.peek(kw::cursor_icon) && !have_cursor_icon {
                                let _: kw::cursor_icon = content2.parse()?;
                                let _: Eq = content2.parse()?;
                                conf.cursor_icon = content2.parse()?;
                                have_cursor_icon = true;
                            } else {
                                return Err(lookahead.error());
                            };

                            if content2.peek(Comma) {
                                let _: Comma = content2.parse()?;
                            }
                        }

                        config = Some(conf);
                    }
                } else {
                    return Err(lookahead.error());
                }

                if content.peek(Comma) {
                    let _: Comma = content.parse()?;
                }
            }
        }

        if !have_config {
            config = Some(WidgetConfig::default());
        }

        Ok(WidgetArgs { config, children })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LayoutType {
    Single,
    Right,
    Left,
    Down,
    Up,
    Grid,
}

impl ToTokens for LayoutType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(match self {
            LayoutType::Single | LayoutType::Grid => unreachable!(),
            LayoutType::Right => quote! { kas::Right },
            LayoutType::Left => quote! { kas::Left },
            LayoutType::Down => quote! { kas::Down },
            LayoutType::Up => quote! { kas::Up },
        })
    }
}

pub struct LayoutArgs {
    pub span: Span,
    pub layout: LayoutType,
    pub area: Option<Ident>,
}

impl Parse for LayoutArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Err(Error::new(
                input.span(),
                "expected attribute parameters: `(..)`",
            ));
        }

        let span = input.span();

        let content;
        let _ = parenthesized!(content in input);

        let lookahead = content.lookahead1();
        let layout = if lookahead.peek(kw::single) {
            let _: kw::single = content.parse()?;
            LayoutType::Single
        } else if lookahead.peek(kw::row) {
            let _: kw::row = content.parse()?;
            LayoutType::Right
        } else if lookahead.peek(kw::right) {
            let _: kw::right = content.parse()?;
            LayoutType::Right
        } else if lookahead.peek(kw::left) {
            let _: kw::left = content.parse()?;
            LayoutType::Left
        } else if lookahead.peek(kw::col) {
            let _: kw::col = content.parse()?;
            LayoutType::Down
        } else if lookahead.peek(kw::column) {
            let _: kw::column = content.parse()?;
            LayoutType::Down
        } else if lookahead.peek(kw::down) {
            let _: kw::down = content.parse()?;
            LayoutType::Down
        } else if lookahead.peek(kw::up) {
            let _: kw::up = content.parse()?;
            LayoutType::Up
        } else if lookahead.peek(kw::grid) {
            let _: kw::grid = content.parse()?;
            LayoutType::Grid
        } else {
            return Err(lookahead.error());
        };

        if content.peek(Comma) {
            let _: Comma = content.parse()?;
        }

        let mut area = None;

        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if area.is_none() && lookahead.peek(kw::area) {
                let _: kw::area = content.parse()?;
                let _: Eq = content.parse()?;
                area = Some(content.parse()?);
            } else {
                return Err(lookahead.error());
            }

            if content.peek(Comma) {
                let _: Comma = content.parse()?;
            }
        }

        Ok(LayoutArgs { span, layout, area })
    }
}

#[derive(Debug)]
pub struct HandlerArgs {
    pub handle: bool,
    pub send: bool,
    pub msg: Type,
    pub substitutions: HashMap<Ident, Type>,
    pub generics: Generics,
}

impl HandlerArgs {
    pub fn new(msg: Type, handle: bool, send: bool) -> Self {
        HandlerArgs {
            handle,
            send,
            msg,
            substitutions: Default::default(),
            generics: Default::default(),
        }
    }
}

impl Default for HandlerArgs {
    fn default() -> Self {
        let msg = parse_quote! { kas::event::VoidMsg };
        HandlerArgs::new(msg, true, true)
    }
}

impl Parse for HandlerArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut have_handle = false;
        let mut have_send = false;
        let mut have_msg = false;
        let mut have_gen = false;
        let mut args = HandlerArgs::default();

        if !input.is_empty() {
            let content;
            let _ = parenthesized!(content in input);

            while !content.is_empty() {
                let lookahead = content.lookahead1();
                if lookahead.peek(kw::noauto) {
                    let _: kw::noauto = content.parse()?;
                    args.handle = false;
                    args.send = false;
                } else if !have_handle && lookahead.peek(kw::handle) {
                    let _: kw::handle = content.parse()?;
                    let _: Eq = content.parse()?;
                    let _: kw::noauto = content.parse()?;
                    have_handle = true;
                    args.handle = false;
                } else if !have_send && lookahead.peek(kw::send) {
                    let _: kw::send = content.parse()?;
                    let _: Eq = content.parse()?;
                    let _: kw::noauto = content.parse()?;
                    have_send = true;
                    args.send = false;
                } else if !have_msg && lookahead.peek(kw::msg) {
                    have_msg = true;
                    let _: kw::msg = content.parse()?;
                    let _: Eq = content.parse()?;
                    args.msg = content.parse()?;
                } else if !have_gen && lookahead.peek(kw::generics) {
                    have_gen = true;
                    let _: kw::generics = content.parse()?;
                    let _: Eq = content.parse()?;

                    // Optionally, substitutions come first
                    while content.peek(Ident) {
                        let ident: Ident = content.parse()?;
                        let _: Token![=>] = content.parse()?;
                        let ty: Type = content.parse()?;
                        args.substitutions.insert(ident, ty);
                        let _: Comma = content.parse()?;
                    }

                    if content.peek(Token![<]) {
                        args.generics = content.parse()?;
                        if content.peek(Token![where]) {
                            args.generics.where_clause = content.parse()?;
                        }
                    } else {
                        return Err(Error::new(
                            content.span(),
                            "expected `< ... > [where ...]` or `T => Substitution ...`",
                        ));
                    }

                    if !content.is_empty() {
                        return Err(Error::new(
                            content.span(),
                            "no more content expected (`generics` must be last parameter)",
                        ));
                    }
                } else {
                    return Err(lookahead.error());
                }

                if content.peek(Comma) {
                    let _: Comma = content.parse()?;
                }
            }
        }

        Ok(args)
    }
}

impl ToTokens for HandlerArgs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        <Token![#]>::default().to_tokens(tokens);
        syn::token::Bracket::default().surround(tokens, |tokens| {
            kw::handler::default().to_tokens(tokens);
            syn::token::Paren::default().surround(tokens, |tokens| {
                if !self.handle {
                    kw::handle::default().to_tokens(tokens);
                    Eq::default().to_tokens(tokens);
                    kw::noauto::default().to_tokens(tokens);
                    Comma::default().to_tokens(tokens);
                }
                if !self.send {
                    kw::send::default().to_tokens(tokens);
                    Eq::default().to_tokens(tokens);
                    kw::noauto::default().to_tokens(tokens);
                    Comma::default().to_tokens(tokens);
                }

                kw::msg::default().to_tokens(tokens);
                Eq::default().to_tokens(tokens);
                self.msg.to_tokens(tokens);
                Comma::default().to_tokens(tokens);

                if !self.substitutions.is_empty()
                    || !self.generics.params.is_empty()
                    || self.generics.where_clause.is_some()
                {
                    kw::generics::default().to_tokens(tokens);
                    Eq::default().to_tokens(tokens);

                    for (k, v) in &self.substitutions {
                        k.to_tokens(tokens);
                        <Token![=>]>::default().to_tokens(tokens);
                        v.to_tokens(tokens);
                        Comma::default().to_tokens(tokens);
                    }

                    self.generics.to_tokens(tokens);
                    if let Some(ref clause) = self.generics.where_clause {
                        if self.generics.params.is_empty() {
                            // generics doesn't print <> in this case
                            <Token![<]>::default().to_tokens(tokens);
                            <Token![>]>::default().to_tokens(tokens);
                        }
                        clause.where_token.to_tokens(tokens);
                        clause.predicates.to_tokens(tokens);
                    }
                }
            });
        });
    }
}

pub enum ChildType {
    Fixed(Type), // fixed type
    // A given type using generics internally
    InternGeneric(Punctuated<GenericParam, Comma>, Type),
    // Generic, optionally with specified handler msg type,
    // optionally with an additional trait bound.
    Generic(Option<Type>, Option<TypeTraitObject>),
}

pub struct WidgetField {
    pub widget_attr: Option<WidgetAttr>,
    pub ident: Option<Ident>,
    pub ty: ChildType,
    pub value: Expr,
}

pub struct MakeWidget {
    // handler attribute
    pub handler: Option<HandlerArgs>,
    // additional attributes
    pub extra_attrs: TokenStream,
    pub generics: Generics,
    pub struct_span: Span,
    // child widgets and data fields
    pub fields: Vec<WidgetField>,
    // impl blocks on the widget
    pub impls: Vec<(Option<TypePath>, Vec<syn::ImplItem>)>,
}

impl Parse for MakeWidget {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut handler = None;
        let mut extra_attrs = TokenStream::new();
        let mut attrs = input.call(Attribute::parse_outer)?;
        for attr in attrs.drain(..) {
            if attr.path == parse_quote! { handler } {
                if handler.is_some() {
                    return Err(Error::new(attr.span(), "duplicate `handler` attribute"));
                }
                handler = Some(syn::parse2(attr.tokens)?);
            } else {
                extra_attrs.append_all(quote! { #attr });
            }
        }

        let s: Token![struct] = input.parse()?;

        let mut generics: syn::Generics = input.parse()?;
        if input.peek(syn::token::Where) {
            generics.where_clause = Some(input.parse()?);
        }

        let content;
        let _ = braced!(content in input);
        let mut fields = vec![];

        while !content.is_empty() {
            fields.push(content.parse::<WidgetField>()?);

            if content.is_empty() {
                break;
            }
            let _: Comma = content.parse()?;
        }

        let mut impls = vec![];
        while !input.is_empty() {
            let _: Impl = input.parse()?;

            let target = if input.peek(Brace) {
                None
            } else {
                Some(input.parse::<TypePath>()?)
            };

            let content;
            let _ = braced!(content in input);
            let mut methods = vec![];

            while !content.is_empty() {
                methods.push(content.parse::<syn::ImplItem>()?);
            }

            impls.push((target, methods));
        }

        Ok(MakeWidget {
            handler,
            extra_attrs,
            generics,
            struct_span: s.span(),
            fields,
            impls,
        })
    }
}

impl Parse for WidgetField {
    fn parse(input: ParseStream) -> Result<Self> {
        let widget_attr = if input.peek(Token![#]) {
            let _: Token![#] = input.parse()?;
            let inner;
            let _ = bracketed!(inner in input);
            let _: kw::widget = inner.parse()?;
            let args = inner.parse::<WidgetAttrArgs>()?;
            Some(WidgetAttr { args })
        } else {
            None
        };

        let ident = {
            let lookahead = input.lookahead1();
            if lookahead.peek(Token![_]) {
                let _: Token![_] = input.parse()?;
                None
            } else if lookahead.peek(Ident) {
                Some(input.parse::<Ident>()?)
            } else {
                return Err(lookahead.error());
            }
        };

        // Note: Colon matches `::` but that results in confusing error messages
        let mut ty = if input.peek(Colon) && !input.peek2(Colon) {
            let _: Colon = input.parse()?;
            if input.peek(For) {
                // internal generic
                let _: For = input.parse()?;

                // copied from syn::Generic's Parse impl
                let _: Token![<] = input.parse()?;

                let mut params = Punctuated::new();
                let mut allow_lifetime_param = true;
                let mut allow_type_param = true;
                loop {
                    if input.peek(Token![>]) {
                        break;
                    }

                    let attrs = input.call(Attribute::parse_outer)?;
                    let lookahead = input.lookahead1();
                    if allow_lifetime_param && lookahead.peek(Lifetime) {
                        params.push_value(GenericParam::Lifetime(LifetimeDef {
                            attrs,
                            ..input.parse()?
                        }));
                    } else if allow_type_param && lookahead.peek(Ident) {
                        allow_lifetime_param = false;
                        params.push_value(GenericParam::Type(TypeParam {
                            attrs,
                            ..input.parse()?
                        }));
                    } else if lookahead.peek(Token![const]) {
                        allow_lifetime_param = false;
                        allow_type_param = false;
                        params.push_value(GenericParam::Const(ConstParam {
                            attrs,
                            ..input.parse()?
                        }));
                    } else {
                        return Err(lookahead.error());
                    }

                    if input.peek(Token![>]) {
                        break;
                    }
                    let punct = input.parse()?;
                    params.push_punct(punct);
                }

                let _: Token![>] = input.parse()?;

                let ty = input.parse()?;
                ChildType::InternGeneric(params, ty)
            } else if input.peek(Impl) {
                // generic with trait bound, optionally with msg type
                let _: Impl = input.parse()?;
                let bound: TypeTraitObject = input.parse()?;
                ChildType::Generic(None, Some(bound))
            } else {
                ChildType::Fixed(input.parse()?)
            }
        } else {
            ChildType::Generic(None, None)
        };

        if input.peek(Token![->]) {
            let arrow: Token![->] = input.parse()?;
            if !widget_attr.is_some() {
                return Err(Error::new(
                    arrow.span(),
                    "can only use `-> Msg` type restriction on widgets",
                ));
            }
            let msg: Type = input.parse()?;
            match &mut ty {
                ChildType::Fixed(_) | ChildType::InternGeneric(_, _) => {
                    return Err(Error::new(
                        arrow.span(),
                        "cannot use `-> Msg` type restriction with fixed `type` or with `for<...> type`",
                    ));
                }
                ChildType::Generic(ref mut gen_r, _) => {
                    *gen_r = Some(msg);
                }
            }
        }

        let _: Eq = input.parse()?;
        let value: Expr = input.parse()?;

        Ok(WidgetField {
            widget_attr,
            ident,
            ty,
            value,
        })
    }
}
