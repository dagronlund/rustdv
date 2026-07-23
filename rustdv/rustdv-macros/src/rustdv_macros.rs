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
    expect_error: Option<String>,
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
            // Pass only if the test fails with this cause — the port of
            // pyuvm's `expect_error=SomeException` (D68).
            "expect_error" => {
                let v = value.ok_or("expect_error needs a value, e.g. expect_error = \"config_not_found\"")?;
                opts.expect_error = Some(strip_quotes(&v));
            }
            other => return Err(format!("unknown #[rustdv::test] option '{other}'")),
        }
    }
    Ok(opts)
}

/// Which of the two front doors this item is (D46).
#[derive(Copy, Clone, PartialEq, Eq)]
enum TestForm {
    /// `async fn tb(ctx: RustdvCtx) -> Result<(), TestError>` — the
    /// cocotb shape, `@cocotb.test()` on a coroutine function.
    Function,
    /// `struct RandomTest;` or `type RandomTest = AluTest<RandomTester>;`
    /// implementing `Component` — the pyuvm shape, `@pyuvm.test()` on a
    /// class.
    Component,
}

/// The item's kind and name: the first top-level `fn` / `struct` / `type`
/// keyword and the identifier after it. Attributes (`#[derive(..)]`) are
/// bracket groups, not top-level idents, so they are skipped for free;
/// `pub` and `async` are idents that simply are not the keyword.
fn find_item(item: &TokenStream) -> Option<(TestForm, String)> {
    let mut form: Option<TestForm> = None;
    for tt in item.clone() {
        if let TokenTree::Ident(i) = tt {
            let s = i.to_string();
            if let Some(f) = form {
                return Some((f, s));
            }
            form = match s.as_str() {
                "fn" => Some(TestForm::Function),
                "struct" | "type" => Some(TestForm::Component),
                _ => None,
            };
        }
    }
    None
}

fn compile_error(msg: &str) -> TokenStream {
    format!("compile_error!({msg:?});").parse().unwrap()
}

/// Both front doors (design-doc §6.1, D46). Registers at link time either
///
/// - `async fn name(ctx: RustdvCtx) -> Result<(), TestError>` — the port of
///   `@cocotb.test()`, decorating a coroutine *function*; or
/// - `struct Name;` / `type Name = ..;` implementing `Component + Default`
///   — the port of `@pyuvm.test()`, decorating a *class*.
///
/// Both expand to the same erased shim, so there is one registry and one
/// execution path behind the two syntaxes.
#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let opts = match parse_test_opts(attr) {
        Ok(o) => o,
        Err(e) => return compile_error(&e),
    };
    let Some((form, item_name)) = find_item(&item) else {
        return compile_error(
            "#[rustdv::test] must be applied to an async fn, a struct, or a type alias",
        );
    };
    let fn_name = item_name;
    let test_name = opts.name.unwrap_or_else(|| fn_name.clone());
    let timeout = match (opts.timeout_time, opts.timeout_unit) {
        (Some(t), Some(u)) => format!("::core::option::Option::Some(({t}u64, \"{u}\"))"),
        (Some(t), None) => format!("::core::option::Option::Some(({t}u64, \"ns\"))"),
        _ => "::core::option::Option::None".to_string(),
    };
    let skip = opts.skip;
    let expect_fail = opts.expect_fail;
    let expect_error = match &opts.expect_error {
        Some(k) => format!("::core::option::Option::Some(\"{k}\")"),
        None => "::core::option::Option::None".to_string(),
    };

    // The two forms differ only in this body: call the function, or build
    // the component and let the phaser drive its whole lifecycle (D51). The
    // struct form no longer calls `run` directly — `run_component_test`
    // runs build → connect → … → run → extract → check → report → final.
    let body = match form {
        TestForm::Function => format!("::std::boxed::Box::pin({fn_name}(ctx))"),
        TestForm::Component => format!(
            r#"::std::boxed::Box::pin(async move {{
            let mut __ctx = ctx;
            let mut __test = <{fn_name} as ::core::default::Default>::default();
            ::rustdv::run_component_test(&mut __test, &mut __ctx).await
        }})"#
        ),
    };

    let reg = format!(
        r#"
const _: () = {{
    fn __rustdv_shim(
        ctx: ::rustdv::RustdvCtx,
    ) -> ::std::pin::Pin<::std::boxed::Box<
        dyn ::std::future::Future<Output = ::core::result::Result<(), ::rustdv::TestError>>,
    >> {{
        {body}
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
        expect_error: {expect_error},
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

/// Does a struct-level `#[component(...)]` (before the `struct` keyword)
/// contain `word`? Field-level attributes come after `struct`, so scanning
/// only the leading tokens keeps them out.
fn struct_attr_contains(input: &TokenStream, word: &str) -> bool {
    let mut prev_hash = false;
    for tt in input.clone() {
        match &tt {
            TokenTree::Ident(i) if i.to_string() == "struct" => return false,
            TokenTree::Punct(p) if p.as_char() == '#' => prev_hash = true,
            TokenTree::Group(g) if prev_hash && g.delimiter() == Delimiter::Bracket => {
                let text = g.stream().to_string();
                if text.starts_with("component") && text.contains(word) {
                    return true;
                }
                prev_hash = false;
            }
            _ => prev_hash = false,
        }
    }
    false
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
    // A unit struct (`struct HelloWorldTest;`) has no brace group at all —
    // ch23's tests are unit structs, since a test with no children has no
    // fields to declare.
    let mut fields_group = None;
    let mut unit_struct = false;
    let mut generics_tokens: Vec<TokenTree> = Vec::new();
    let mut depth = 0i32;
    for tt in iter {
        match &tt {
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace && depth == 0 => {
                fields_group = Some(g.clone());
                break;
            }
            TokenTree::Punct(p) if p.as_char() == ';' && depth == 0 => {
                unit_struct = true;
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
    if unit_struct {
        return Ok((name, impl_generics, type_params, Vec::new()));
    }
    let group = fields_group
        .ok_or("#[derive(Component)] requires named fields, or a unit struct")?;

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
/// registration (R5) — but R5 is reversed: the factory returns in ch29,
/// and this derive is where its registration will land. The impl is
/// hand-writable; the derive is convenience.
#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    // A struct-level `#[component(no_factory)]` opts out of universal factory
    // registration — for components that are not `Default` (they take
    // constructor arguments) and so cannot be built by name. Detected before
    // the `struct` keyword to distinguish it from field-level attributes.
    let no_factory = struct_attr_contains(&input, "no_factory");

    let (name, impl_generics, type_params, fields) = match parse_struct(input) {
        Ok(v) => v,
        Err(e) => return compile_error(&e),
    };

    let mut visits = String::new();
    let mut resolves = String::new();
    for f in fields.iter().filter(|f| f.is_child) {
        let fname = &f.name;
        let ty = f.ty.trim_start();
        if ty.starts_with("AnyComp") {
            // A factory slot (D75): reach through to the held component if
            // present, and let it resolve its override during the walk.
            visits.push_str(&format!(
                "if let ::core::option::Option::Some(__c) = self.{fname}.as_node_mut() {{ __out.push((::std::string::String::from(\"{fname}\"), __c)); }}\n"
            ));
            resolves.push_str(&format!("self.{fname}.resolve(__ctx, \"{fname}\");\n"));
        } else if ty.starts_with("Option") {
            // "declared but not yet built": a child created during `build`
            // (D6) appears here only once it is `Some`.
            visits.push_str(&format!(
                "if let ::core::option::Option::Some(__c) = &mut self.{fname} {{ __out.push((::std::string::String::from(\"{fname}\"), __c as &mut (dyn ::rustdv::ComponentNode + 'static))); }}\n"
            ));
        } else if ty.starts_with("Vec") {
            visits.push_str(&format!(
                "for (__i, __c) in self.{fname}.iter_mut().enumerate() {{ __out.push((::std::format!(\"{fname}[{{}}]\", __i), __c as &mut (dyn ::rustdv::ComponentNode + 'static))); }}\n"
            ));
        } else {
            visits.push_str(&format!(
                "__out.push((::std::string::String::from(\"{fname}\"), &mut self.{fname} as &mut (dyn ::rustdv::ComponentNode + 'static)));\n"
            ));
        }
    }

    // A resolver only if there is at least one factory slot.
    let resolve_impl = if resolves.is_empty() {
        String::new()
    } else {
        format!(
            "    fn resolve_children(&mut self, __ctx: &::rustdv::RustdvCtx) {{\n        {resolves}    }}\n"
        )
    };

    // Universal registration (D73): enrol non-generic components by name so
    // the factory can build them by string. Generic components are skipped —
    // a `static` cannot be generic, and their monomorphs are not by-name
    // targets.
    let registration = if type_params.is_empty() && !no_factory {
        format!(
            r#"
const _: () = {{
    fn __rustdv_comp_name() -> &'static str {{ "{name}" }}
    fn __rustdv_comp_make() -> ::std::boxed::Box<dyn ::rustdv::ComponentNode> {{
        ::std::boxed::Box::new(<{name} as ::core::default::Default>::default())
    }}
    #[used]
    #[cfg_attr(not(target_vendor = "apple"), link_section = "rustdv_comps")]
    #[cfg_attr(target_vendor = "apple", link_section = "__DATA,rustdv_comps")]
    static __RUSTDV_COMP_REG: &::rustdv::ComponentReg = &::rustdv::ComponentReg {{
        name: __rustdv_comp_name,
        make: __rustdv_comp_make,
    }};
}};
"#
        )
    } else {
        String::new()
    };

    let out = format!(
        r#"
impl {impl_generics} ::rustdv::ComponentNode for {name} {type_params} {{
    fn node_name(&self) -> &'static str {{ "{name}" }}
    fn children_mut(&mut self) -> ::std::vec::Vec<(::std::string::String, &mut (dyn ::rustdv::ComponentNode + 'static))> {{
        let mut __out: ::std::vec::Vec<(::std::string::String, &mut (dyn ::rustdv::ComponentNode + 'static))> = ::std::vec::Vec::new();
        {visits}
        __out
    }}
{resolve_impl}}}
{registration}"#
    );
    out.parse().expect("rustdv-macros: generated ComponentNode impl failed to parse")
}
