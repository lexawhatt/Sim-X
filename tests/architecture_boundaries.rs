use std::{fs, path::Path};

#[test]
fn foundation_and_domains_have_no_desktop_or_presentation_dependencies() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut rust_sources = Vec::new();
    collect_rust_sources(&manifest_dir.join("src/foundation"), &mut rust_sources);
    collect_rust_sources(&manifest_dir.join("src/domains"), &mut rust_sources);

    assert!(
        !rust_sources.is_empty(),
        "architecture scan found no Rust sources"
    );
    for source_path in rust_sources {
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("could not read {}: {error}", source_path.display()));
        let tokens = source_tokens(&source);
        assert!(
            !contains_feature_gate(&tokens),
            "{} uses a feature-gate inside scientific code; this could hide a desktop dependency from the required no-default-features build",
            source_path.display()
        );
        assert!(
            !references_root_module_tokens(&tokens, "presentation"),
            "{} imports crate::presentation",
            source_path.display()
        );
        for forbidden in ["sim_engine", "winit", "rodio", "wgpu"] {
            assert!(
                !references_external_crate(&tokens, forbidden),
                "{} contains forbidden dependency identifier {forbidden}",
                source_path.display()
            );
        }
    }
}

#[test]
fn foundation_does_not_depend_on_domains() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut rust_sources = Vec::new();
    collect_rust_sources(&manifest_dir.join("src/foundation"), &mut rust_sources);

    assert!(
        !rust_sources.is_empty(),
        "foundation architecture scan found no Rust sources"
    );
    for source_path in rust_sources {
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("could not read {}: {error}", source_path.display()));
        assert!(
            !references_root_module(&source, "domains"),
            "{} imports crate::domains; foundation must remain domain-neutral",
            source_path.display()
        );
    }
}

#[test]
fn physics_subdomains_do_not_import_their_siblings() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let physics = manifest_dir.join("src/domains/phys");
    let subdomains = [
        "mechanics",
        "thermodynamics",
        "waves_optics",
        "electromagnetism",
    ];

    for owner in subdomains {
        let mut sources = Vec::new();
        collect_rust_sources(&physics.join(owner), &mut sources);
        for source_path in sources {
            let source = fs::read_to_string(&source_path).unwrap_or_else(|error| {
                panic!("could not read {}: {error}", source_path.display())
            });
            let tokens = source_tokens(&source);
            for sibling in subdomains
                .into_iter()
                .filter(|candidate| *candidate != owner)
            {
                assert!(
                    !references_physics_subdomain(&tokens, sibling),
                    "{} imports sibling Phys subdomain {sibling}; bridges belong to the app boundary",
                    source_path.display()
                );
            }
        }
    }
}

#[test]
fn desktop_crates_are_imported_only_by_their_approved_adapters() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_root = manifest_dir.join("src");
    let renderer_adapter = source_root.join("presentation/sim_engine");
    let audio_adapter = source_root.join("presentation/audio");
    let mut sources = Vec::new();
    collect_rust_sources(&source_root, &mut sources);

    for source_path in sources {
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("could not read {}: {error}", source_path.display()));
        let tokens = source_tokens(&source);
        if references_external_crate(&tokens, "sim_engine") {
            assert!(
                source_path.starts_with(&renderer_adapter),
                "{} imports sim_engine outside the approved adapter",
                source_path.display()
            );
        }
        if references_external_crate(&tokens, "rodio") {
            assert!(
                source_path.starts_with(&audio_adapter),
                "{} imports rodio outside the approved audio adapter",
                source_path.display()
            );
        }
    }
}

#[test]
fn root_module_reference_detection_covers_supported_import_forms() {
    for source in [
        "use crate::domains::phys;",
        "use crate :: domains :: phys;",
        "use crate::{domains::phys, foundation::ApiVersion};",
        "use crate::{foundation::ApiVersion, domains::phys};",
        "let value = crate::domains::phys::VALUE;",
        "#[cfg(target_os = \"linux\")] use crate :: { foundation::ApiVersion, domains as model };",
    ] {
        assert!(
            references_root_module(source, "domains"),
            "architecture scanner missed {source:?}"
        );
    }

    for source in [
        "use crate::foundation::ApiVersion;",
        "use external_crate::domains::Domain;",
        "const LABEL: &str = \"domains\";",
        "// use crate::domains::phys;",
        "const RAW: &str = r#\"crate::domains::phys\"#;",
    ] {
        assert!(
            !references_root_module(source, "domains"),
            "architecture scanner rejected unrelated source {source:?}"
        );
    }
}

fn references_root_module(source: &str, module: &str) -> bool {
    references_root_module_tokens(&source_tokens(source), module)
}

#[test]
fn desktop_identifier_detection_covers_alias_extern_whitespace_and_cfg() {
    for source in [
        "use sim_engine as renderer;",
        "extern crate sim_engine as renderer;",
        "let _ = sim_engine :: Camera2d::default();",
        "#[cfg(target_os = \"linux\")] use :: winit as windowing;",
        "#[cfg(any())] extern crate rodio;",
        "use wgpu as graphics;",
    ] {
        let tokens = source_tokens(source);
        assert!(
            ["sim_engine", "winit", "rodio", "wgpu"]
                .into_iter()
                .any(|name| references_external_crate(&tokens, name)),
            "desktop dependency detector missed {source:?}"
        );
    }

    for source in [
        "const MESSAGE: &str = \"use sim_engine as renderer\";",
        "const RAW: &str = r#\"extern crate rodio\"#;",
        "// use wgpu as graphics;",
        "/* extern crate winit; */ fn scientific_value() {}",
        "mod sim_engine;",
        "use crate::presentation::sim_engine::SimEnginePresenter;",
    ] {
        let tokens = source_tokens(source);
        assert!(
            ["sim_engine", "winit", "rodio", "wgpu"]
                .into_iter()
                .all(|name| !references_external_crate(&tokens, name)),
            "desktop dependency detector rejected non-code text {source:?}"
        );
    }
}

#[test]
fn sibling_detector_covers_absolute_relative_alias_and_target_gates() {
    for source in [
        "use crate :: domains :: phys :: thermodynamics as heat;",
        "#[cfg(target_os = \"linux\")] use super :: waves_optics as waves;",
        "use crate::domains::phys::{mechanics as motion, waves_optics};",
        "use super::{electromagnetism as fields};",
    ] {
        let tokens = source_tokens(source);
        assert!(
            [
                "mechanics",
                "thermodynamics",
                "waves_optics",
                "electromagnetism",
            ]
            .into_iter()
            .any(|sibling| references_physics_subdomain(&tokens, sibling)),
            "sibling detector missed {source:?}"
        );
    }

    for source in [
        "use crate::foundation::units::Meters;",
        "use external_crate::mechanics as mechanics_library;",
        "const LABEL: &str = \"super::thermodynamics\";",
        "// use super::electromagnetism;",
    ] {
        let tokens = source_tokens(source);
        assert!(
            [
                "mechanics",
                "thermodynamics",
                "waves_optics",
                "electromagnetism",
            ]
            .into_iter()
            .all(|sibling| !references_physics_subdomain(&tokens, sibling)),
            "sibling detector rejected unrelated source {source:?}"
        );
    }
}

#[test]
fn feature_gate_detector_covers_spacing_comments_and_nested_predicates() {
    for source in [
        "#[cfg(feature=\"desktop\")] fn hidden() {}",
        "#[cfg(feature /* deliberately separated */ = \"desktop\")] fn hidden() {}",
        "#[cfg(any(unix, feature = \"desktop\"))] fn hidden() {}",
        "#[cfg_attr(feature = \"desktop\", derive(Debug))] struct Hidden;",
        "#![cfg(feature = \"desktop\")]",
    ] {
        assert!(
            contains_feature_gate(&source_tokens(source)),
            "feature-gate detector missed {source:?}"
        );
    }

    for source in [
        "const MESSAGE: &str = \"#[cfg(feature = \\\"desktop\\\")]\";",
        "// #[cfg(feature = \"desktop\")]\nfn scientific_value() {}",
        "#[cfg(target_os = \"linux\")] fn scientific_value() {}",
        "fn feature(value: bool) { let _ = value; }",
    ] {
        assert!(
            !contains_feature_gate(&source_tokens(source)),
            "feature-gate detector rejected unrelated source {source:?}"
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceToken<'source> {
    Identifier(&'source str),
    PathSeparator,
    Pound,
    Bang,
    OpenBracket,
    CloseBracket,
    OpenParenthesis,
    CloseParenthesis,
    Equals,
    OpenBrace,
    CloseBrace,
    Semicolon,
    Other,
}

fn source_tokens(source: &str) -> Vec<SourceToken<'_>> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if let Some(end) = raw_string_end(bytes, index) {
            index = end;
            continue;
        }
        match bytes[index] {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index = block_comment_end(bytes, index + 2);
            }
            b'"' => index = quoted_string_end(bytes, index + 1),
            b'b' | b'c' if bytes.get(index + 1) == Some(&b'"') => {
                index = quoted_string_end(bytes, index + 2);
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                let start = index;
                index += 1;
                while bytes
                    .get(index)
                    .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                {
                    index += 1;
                }
                tokens.push(SourceToken::Identifier(&source[start..index]));
            }
            b':' if bytes.get(index + 1) == Some(&b':') => {
                tokens.push(SourceToken::PathSeparator);
                index += 2;
            }
            b'#' => {
                tokens.push(SourceToken::Pound);
                index += 1;
            }
            b'!' => {
                tokens.push(SourceToken::Bang);
                index += 1;
            }
            b'[' => {
                tokens.push(SourceToken::OpenBracket);
                index += 1;
            }
            b']' => {
                tokens.push(SourceToken::CloseBracket);
                index += 1;
            }
            b'(' => {
                tokens.push(SourceToken::OpenParenthesis);
                index += 1;
            }
            b')' => {
                tokens.push(SourceToken::CloseParenthesis);
                index += 1;
            }
            b'=' => {
                tokens.push(SourceToken::Equals);
                index += 1;
            }
            b'{' => {
                tokens.push(SourceToken::OpenBrace);
                index += 1;
            }
            b'}' => {
                tokens.push(SourceToken::CloseBrace);
                index += 1;
            }
            b';' => {
                tokens.push(SourceToken::Semicolon);
                index += 1;
            }
            byte if byte.is_ascii_whitespace() => index += 1,
            _ => {
                tokens.push(SourceToken::Other);
                index += 1;
            }
        }
    }
    tokens
}

fn contains_feature_gate(tokens: &[SourceToken<'_>]) -> bool {
    tokens.iter().enumerate().any(|(start, token)| {
        if *token != SourceToken::Pound {
            return false;
        }
        let bracket = if tokens.get(start + 1) == Some(&SourceToken::Bang) {
            start + 2
        } else {
            start + 1
        };
        if tokens.get(bracket) != Some(&SourceToken::OpenBracket)
            || !matches!(
                tokens.get(bracket + 1),
                Some(SourceToken::Identifier("cfg" | "cfg_attr"))
            )
            || tokens.get(bracket + 2) != Some(&SourceToken::OpenParenthesis)
        {
            return false;
        }

        let mut parenthesis_depth = 1_usize;
        let mut cursor = bracket + 3;
        while cursor < tokens.len() && parenthesis_depth != 0 {
            match tokens[cursor] {
                SourceToken::OpenParenthesis => parenthesis_depth += 1,
                SourceToken::CloseParenthesis => parenthesis_depth -= 1,
                SourceToken::Identifier("feature")
                    if tokens.get(cursor + 1) == Some(&SourceToken::Equals) =>
                {
                    return true;
                }
                _ => {}
            }
            cursor += 1;
        }
        false
    })
}

fn raw_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut prefix_end = start;
    if bytes.get(prefix_end) == Some(&b'b') {
        prefix_end += 1;
    }
    if bytes.get(prefix_end) != Some(&b'r') {
        return None;
    }
    let mut quote = prefix_end + 1;
    while bytes.get(quote) == Some(&b'#') {
        quote += 1;
    }
    if bytes.get(quote) != Some(&b'"') {
        return None;
    }
    let hashes = quote - prefix_end - 1;
    let mut cursor = quote + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && bytes.get(cursor + 1..cursor + 1 + hashes) == Some(&bytes[quote - hashes..quote])
        {
            return Some(cursor + 1 + hashes);
        }
        cursor += 1;
    }
    Some(bytes.len())
}

fn quoted_string_end(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = (index + 2).min(bytes.len()),
            b'"' => return index + 1,
            _ => index += 1,
        }
    }
    bytes.len()
}

fn block_comment_end(bytes: &[u8], mut index: usize) -> usize {
    let mut depth = 1_usize;
    while index < bytes.len() && depth != 0 {
        if bytes.get(index..index + 2) == Some(b"/*") {
            depth += 1;
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"*/") {
            depth -= 1;
            index += 2;
        } else {
            index += 1;
        }
    }
    index
}

fn references_external_crate(tokens: &[SourceToken<'_>], crate_name: &str) -> bool {
    let extern_declaration = tokens.windows(3).any(|window| {
        window
            == [
                SourceToken::Identifier("extern"),
                SourceToken::Identifier("crate"),
                SourceToken::Identifier(crate_name),
            ]
    });
    let imported_at_root = use_statements(tokens)
        .any(|statement| source_identifiers(statement).first().copied() == Some(crate_name));
    let root_path = tokens.iter().enumerate().any(|(index, token)| {
        if *token != SourceToken::Identifier(crate_name)
            || tokens.get(index + 1) != Some(&SourceToken::PathSeparator)
            || inside_use_statement(tokens, index)
        {
            return false;
        }
        tokens.get(index.wrapping_sub(1)) != Some(&SourceToken::PathSeparator)
            || !matches!(
                tokens.get(index.wrapping_sub(2)),
                Some(SourceToken::Identifier(_))
            )
    });

    extern_declaration || imported_at_root || root_path
}

fn inside_use_statement(tokens: &[SourceToken<'_>], index: usize) -> bool {
    let statement_start = tokens[..index]
        .iter()
        .rposition(|token| *token == SourceToken::Semicolon)
        .map_or(0, |semicolon| semicolon + 1);
    tokens[statement_start..index].contains(&SourceToken::Identifier("use"))
}

fn references_root_module_tokens(tokens: &[SourceToken<'_>], module: &str) -> bool {
    contains_path(tokens, &["crate", module])
        || use_statements(tokens).any(|statement| {
            let identifiers = source_identifiers(statement);
            identifiers.first() == Some(&"crate") && identifiers.contains(&module)
        })
}

fn references_physics_subdomain(tokens: &[SourceToken<'_>], subdomain: &str) -> bool {
    contains_path(tokens, &["domains", "phys", subdomain])
        || contains_path(tokens, &["super", subdomain])
        || use_statements(tokens).any(|statement| {
            let identifiers = source_identifiers(statement);
            contains_ordered(&identifiers, &["domains", "phys", subdomain])
                || (identifiers.first() == Some(&"super") && identifiers.contains(&subdomain))
        })
}

fn contains_path(tokens: &[SourceToken<'_>], components: &[&str]) -> bool {
    tokens.iter().enumerate().any(|(start, token)| {
        if *token != SourceToken::Identifier(components[0]) {
            return false;
        }
        components
            .iter()
            .skip(1)
            .enumerate()
            .all(|(offset, component)| {
                tokens.get(start + offset * 2 + 1) == Some(&SourceToken::PathSeparator)
                    && tokens.get(start + offset * 2 + 2)
                        == Some(&SourceToken::Identifier(component))
            })
    })
}

fn use_statements<'tokens, 'source>(
    tokens: &'tokens [SourceToken<'source>],
) -> impl Iterator<Item = &'tokens [SourceToken<'source>]> {
    tokens
        .split(|token| *token == SourceToken::Semicolon)
        .filter_map(|statement| {
            statement
                .iter()
                .position(|token| *token == SourceToken::Identifier("use"))
                .map(|use_index| &statement[use_index + 1..])
        })
}

fn source_identifiers<'source>(tokens: &[SourceToken<'source>]) -> Vec<&'source str> {
    let mut identifiers = Vec::new();
    let mut skip_alias = false;
    for token in tokens {
        let SourceToken::Identifier(identifier) = token else {
            continue;
        };
        if *identifier == "as" {
            skip_alias = true;
        } else if skip_alias {
            skip_alias = false;
        } else {
            identifiers.push(*identifier);
        }
    }
    identifiers
}

fn contains_ordered(identifiers: &[&str], expected: &[&str]) -> bool {
    let mut expected = expected.iter();
    let mut next = expected.next();
    for identifier in identifiers {
        if next == Some(identifier) {
            next = expected.next();
        }
    }
    next.is_none()
}

fn collect_rust_sources(directory: &Path, output: &mut Vec<std::path::PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("could not scan {}: {error}", directory.display()))
        .map(|entry| entry.expect("directory entry must be readable").path())
        .collect();
    entries.sort();

    for path in entries {
        if path.is_dir() {
            collect_rust_sources(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}
