//! Proc macros (design-doc §6). Two macros only, per the governing
//! principle — a macro is justified only where Python used runtime
//! dynamism Rust lacks:
//!
//! - `#[rustdv::test]` (§6.1): registers the annotated `async fn` in the
//!   link-time test registry (the `inventory`/`linkme` technique, hand
//!   rolled for ELF: `#[link_section]` + `__start_`/`__stop_` symbols).
//! - `#[derive(Component)]` (§6.3): generates the `ComponentNode`
//!   traversal over `#[component(child)]` fields (`T`, `Option<T>`,
//!   `Vec<T>`).
//!
//! **Implementation note (STATUS.md):** the zero-dependency constraint
//! rules out syn/quote, so parsing walks raw token trees and code
//! generation goes through string formatting + `.parse()`. Supported
//! grammar is deliberately narrow (plain fns, non-generic structs with
//! named fields); unsupported shapes produce compile errors.

use proc_macro::{Delimiter, TokenStream, TokenTree};

// ===========================================================================
// #[rustdv::test]
// ===========================================================================

#[derive(Default)]
struct TestOpts {
    name: Option<String>,
    timeout_time: Option<u64>,
    timeout_unit: Option<String>,
    skip: bool,
    expect_fail: bool,
}

fn strip_quotes(s: &str) -> String {
    s.trim_matches('"').to_string()
}

fn parse_test_opts(attr: TokenStream) -> Result<TestOpts, String> {
    let mut opts = TestOpts::default();
    let mut iter = attr.into_iter().peekable();
    while let Some(tt) = iter.next() {
        let key = match &tt {
            TokenTree::Ident(i) => i.to_string(),
            TokenTree::Punct(p) if p.as_char() == ',' => continue,
            other => return Err(format!("unexpected token in #[rustdv::test(...)]: {other}")),
        };
        // Optional `= value`
        let mut value: Option<String> = None;
        if let Some(TokenTree::Punct(p)) = iter.peek() {
            if p.as_char() == '=' {
                iter.next(); // consume '='
                match iter.next() {
                    Some(TokenTree::Literal(l)) => value = Some(l.to_string()),
                    Some(TokenTree::Ident(i)) => value = Some(i.to_string()),
                    other => return Err(format!("expected value after '{key} =', got {other:?}")),
                }
            }
        }
        match key.as_str() {
            "name" => opts.name = value.map(|v| strip_quotes(&v)),
            "timeout_time" => {
                let v = value.ok_or("timeout_time needs a value")?;
                opts.timeout_time =
                    Some(v.parse::<u64>().map_err(|_| format!("bad timeout_time '{v}'"))?);
            }
            "timeout_unit" => opts.timeout_unit = value.map(|v| strip_quotes(&v)),
            "skip" => opts.skip = value.map(|v| v == "true").unwrap_or(true),
            "expect_fail" => opts.expect_fail = value.map(|v| v == "true").unwrap_or(true),
            other => return Err(format!("unknown #[rustdv::test] option '{other}'")),
        }
    }
    Ok(opts)
}

/// The identifier following the `fn` keyword at top level of the item.
fn find_fn_name(item: &TokenStream) -> Option<String> {
    let mut saw_fn = false;
    for tt in item.clone() {
        match tt {
            TokenTree::Ident(i) => {
                let s = i.to_string();
                if saw_fn {
                    return Some(s);
                }
                if s == "fn" {
                    saw_fn = true;
                }
            }
            _ => {
                if saw_fn {
                    return None;
                }
            }
        }
    }
    None
}

fn compile_error(msg: &str) -> TokenStream {
    format!("compile_error!({msg:?});").parse().unwrap()
}

/// Port of `@cocotb.test()` (design-doc §6.1). Registers the annotated
/// `async fn name(ctx: TestCtx) -> Result<(), TestError>` at link time.
#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let opts = match parse_test_opts(attr) {
        Ok(o) => o,
        Err(e) => return compile_error(&e),
    };
    let Some(fn_name) = find_fn_name(&item) else {
        return compile_error("#[rustdv::test] must be applied to an async fn");
    };
    let test_name = opts.name.unwrap_or_else(|| fn_name.clone());
    let timeout = match (opts.timeout_time, opts.timeout_unit) {
        (Some(t), Some(u)) => format!("::core::option::Option::Some(({t}u64, \"{u}\"))"),
        (Some(t), None) => format!("::core::option::Option::Some(({t}u64, \"ns\"))"),
        _ => "::core::option::Option::None".to_string(),
    };
    let skip = opts.skip;
    let expect_fail = opts.expect_fail;

    let reg = format!(
        r#"
const _: () = {{
    fn __rustdv_shim(
        ctx: ::rustdv::TestCtx,
    ) -> ::std::pin::Pin<::std::boxed::Box<
        dyn ::std::future::Future<Output = ::core::result::Result<(), ::rustdv::TestError>>,
    >> {{
        ::std::boxed::Box::pin({fn_name}(ctx))
    }}
    #[used]
    #[cfg_attr(not(target_vendor = "apple"), link_section = "rustdv_tests")]
    #[cfg_attr(target_vendor = "apple", link_section = "__DATA,rustdv_tests")]
    static __RUSTDV_TEST_REG: &'static ::rustdv::TestRegistration = &::rustdv::TestRegistration {{
        name: "{test_name}",
        module: ::core::module_path!(),
        file: ::core::file!(),
        line: ::core::line!(),
        run: __rustdv_shim,
        timeout: {timeout},
        skip: {skip},
        expect_fail: {expect_fail},
    }};
}};
"#
    );

    let mut out = item;
    out.extend(reg.parse::<TokenStream>().expect("rustdv-macros: generated code failed to parse"));
    out
}

// ===========================================================================
// #[derive(Component)]
// ===========================================================================

struct Field {
    name: String,
    ty: String,
    is_child: bool,
}

/// Parse `struct Name { ... }` from the derive input token stream.
/// Supported: structs with named fields, including simple generics
/// (`struct Env<T: Tester + 'static> { ... }`).
fn parse_struct(input: TokenStream) -> Result<(String, String, String, Vec<Field>), String> {
    let mut iter = input.into_iter().peekable();
    let mut struct_name: Option<String> = None;

    // Find `struct` then its name, then the brace group.
    while let Some(tt) = iter.next() {
        if let TokenTree::Ident(i) = &tt {
            if i.to_string() == "struct" {
                match iter.next() {
                    Some(TokenTree::Ident(n)) => {
                        struct_name = Some(n.to_string());
                        break;
                    }
                    _ => return Err("expected struct name".into()),
                }
            }
        }
    }
    let name = struct_name.ok_or("#[derive(Component)] supports only structs")?;

    // Capture optional generics `<...>` (with bounds), then the brace group.
    let mut fields_group = None;
    let mut generics_tokens: Vec<TokenTree> = Vec::new();
    let mut depth = 0i32;
    for tt in iter {
        match &tt {
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace && depth == 0 => {
                fields_group = Some(g.clone());
                break;
            }
            TokenTree::Punct(p) if p.as_char() == '<' => {
                depth += 1;
                generics_tokens.push(tt.clone());
                continue;
            }
            TokenTree::Punct(p) if p.as_char() == '>' => {
                depth -= 1;
                generics_tokens.push(tt.clone());
                continue;
            }
            _ => {}
        }
        if depth > 0 {
            generics_tokens.push(tt.clone());
        }
    }
    // impl generics: verbatim (`<T: Tester + 'static>`); type params: names only.
    let impl_generics: String = {
        let mut out = String::new();
        for t in &generics_tokens {
            let text = t.to_string();
            if !out.is_empty() && !out.ends_with('\'') {
                out.push(' ');
            }
            out.push_str(&text);
        }
        out
    };
    let type_params = {
        // First ident (or lifetime) of each comma-separated part at depth 1.
        let mut params: Vec<String> = Vec::new();
        let mut d = 0i32;
        let mut take_next_ident = true;
        let mut lifetime = false;
        for t in &generics_tokens {
            match t {
                TokenTree::Punct(p) if p.as_char() == '<' => d += 1,
                TokenTree::Punct(p) if p.as_char() == '>' => d -= 1,
                TokenTree::Punct(p) if p.as_char() == ',' && d == 1 => take_next_ident = true,
                TokenTree::Punct(p) if p.as_char() == '\'' && d == 1 && take_next_ident => {
                    lifetime = true
                }
                TokenTree::Ident(i) if d == 1 && take_next_ident => {
                    let word = i.to_string();
                    if word == "const" {
                        continue; // the const param's name is the next ident
                    }
                    params.push(if lifetime { format!("'{word}") } else { word });
                    take_next_ident = false;
                    lifetime = false;
                }
                _ => {}
            }
        }
        if params.is_empty() { String::new() } else { format!("< {} >", params.join(" , ")) }
    };
    let group = fields_group.ok_or("#[derive(Component)] requires named fields")?;

    // Split the group's tokens into fields at top-level commas.
    let mut fields = Vec::new();
    let mut pending_child = false;
    let mut current: Vec<TokenTree> = Vec::new();

    let mut toks = group.stream().into_iter().peekable();
    let mut angle_depth = 0i32;
    while let Some(tt) = toks.next() {
        match &tt {
            TokenTree::Punct(p) if p.as_char() == '<' => angle_depth += 1,
            TokenTree::Punct(p) if p.as_char() == '>' => angle_depth -= 1,
            TokenTree::Punct(p) if p.as_char() == '#' => {
                // attribute: #[ ... ]
                if let Some(TokenTree::Group(g)) = toks.peek() {
                    if g.delimiter() == Delimiter::Bracket {
                        let text = g.stream().to_string();
                        if text.starts_with("component") && text.contains("child") {
                            pending_child = true;
                        }
                        toks.next(); // consume the bracket group
                        continue;
                    }
                }
            }
            TokenTree::Punct(p) if p.as_char() == ',' && angle_depth == 0 => {
                if !current.is_empty() {
                    fields.push(make_field(&current, pending_child)?);
                    current.clear();
                    pending_child = false;
                }
                continue;
            }
            _ => {}
        }
        current.push(tt);
    }
    if !current.is_empty() {
        fields.push(make_field(&current, pending_child)?);
    }

    Ok((name, impl_generics, type_params, fields))
}

/// From tokens like `pub name : Type ...` extract name and type text.
fn make_field(tokens: &[TokenTree], is_child: bool) -> Result<Field, String> {
    let mut name = None;
    let mut colon_at = None;
    for (i, tt) in tokens.iter().enumerate() {
        if let TokenTree::Punct(p) = tt {
            if p.as_char() == ':' && colon_at.is_none() {
                colon_at = Some(i);
                break;
            }
        }
    }
    let colon = colon_at.ok_or("field without ':' (tuple structs unsupported)")?;
    // The ident immediately before ':' is the field name (skips pub/pub(..)).
    for tt in tokens[..colon].iter().rev() {
        if let TokenTree::Ident(i) = tt {
            name = Some(i.to_string());
            break;
        }
    }
    let name = name.ok_or("could not find field name")?;
    let ty: String = tokens[colon + 1..].iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" ");
    Ok(Field { name, ty, is_child })
}

/// Generates the `ComponentNode` impl (design-doc §6.3, revised per R2):
/// traversal of `#[component(child)]` fields, including `Option<T>` and
/// `Vec<T>`; names synthesized from field names. Emits **no** factory
/// registration (R5). The impl is hand-writable; the derive is convenience.
#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let (name, impl_generics, type_params, fields) = match parse_struct(input) {
        Ok(v) => v,
        Err(e) => return compile_error(&e),
    };

    let mut visits = String::new();
    for f in fields.iter().filter(|f| f.is_child) {
        let fname = &f.name;
        let ty = f.ty.trim_start();
        if ty.starts_with("Option") {
            visits.push_str(&format!(
                "if let ::core::option::Option::Some(__c) = &mut self.{fname} {{ f(\"{fname}\", __c); }}\n"
            ));
        } else if ty.starts_with("Vec") {
            visits.push_str(&format!(
                "for (__i, __c) in self.{fname}.iter_mut().enumerate() {{ let __n = ::std::format!(\"{fname}[{{}}]\", __i); f(&__n, __c); }}\n"
            ));
        } else {
            visits.push_str(&format!("f(\"{fname}\", &mut self.{fname});\n"));
        }
    }

    let out = format!(
        r#"
impl {impl_generics} ::rustdv::ComponentNode for {name} {type_params} {{
    fn node_name(&self) -> &'static str {{ "{name}" }}
    fn visit_children(&mut self, f: &mut dyn FnMut(&str, &mut dyn ::rustdv::ComponentNode)) {{
        let _ = &f;
        {visits}
    }}
}}
"#
    );
    out.parse().expect("rustdv-macros: generated ComponentNode impl failed to parse")
}
