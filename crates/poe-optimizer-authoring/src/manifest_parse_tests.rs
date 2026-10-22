use crate::{
    MAX_MANIFEST_DEPTH, ManifestErrorKind, ManifestLimits, manifest_parse::parse_document,
};

fn parse(source: &str) -> crate::manifest_parse::Document {
    parse_document(source, ManifestLimits::default()).unwrap()
}

fn fails(source: &str, limits: ManifestLimits, expected: ManifestErrorKind) {
    let failure = parse_document(source, limits).unwrap_err();
    assert_eq!(
        failure.kind, expected,
        "source: {source:?}; error: {failure}"
    );
}

#[test]
fn manifest_parser_keeps_ordered_structure_and_kdl_v2_string_forms() {
    let document = parse(
        r####"// Ordinary KDL syntax, decoded by the upstream parser.
root "first" key=bare {
    /* outer /* nested */ end */ child #"literal { } /- \n"#;
    text """
        first
        second
        """;
    /- omitted "value";
    last /- old="unused" current="kept";
}
sibling "last"
"####,
    );
    assert_eq!(document.roots, [0, 4]);
    assert_eq!(document.nodes.len(), 5);
    assert_eq!(document.nodes[0].name, "root");
    assert_eq!(document.nodes[0].arguments, ["first"]);
    assert_eq!(document.nodes[0].properties["key"], "bare");
    assert_eq!(document.nodes[0].children, [1, 2, 3]);
    assert_eq!(document.nodes[1].arguments, [r"literal { } /- \n"]);
    assert_eq!(document.nodes[2].arguments, ["first\nsecond"]);
    assert_eq!(document.nodes[3].properties.len(), 1);
    assert_eq!(document.nodes[3].properties["current"], "kept");
    assert_eq!(document.nodes[4].arguments, ["last"]);
}

#[test]
fn manifest_escaped_strings_decode_exactly_in_optimized_builds() {
    // just-kdl 0.3.0 accidentally put the inline UTF-8 write inside a debug
    // assertion. Keep this regression in the same suite with assertions off.
    let document = parse(r#""na\u{6d}e" "a\n" "\u{e9}" "\u{1f642}" "\"\\\s\t" p="\u{3bb}""#);
    assert_eq!(document.nodes[0].name, "name");
    assert_eq!(document.nodes[0].arguments, ["a\n", "é", "🙂", "\"\\ \t"]);
    assert_eq!(document.nodes[0].properties["p"], "λ");
    assert!(
        !document.nodes[0]
            .arguments
            .iter()
            .any(|value| value.contains('\0'))
    );
    // Exercise the inline-to-heap transition as well as entirely inline tokens.
    let document = parse(r#"n "1234567890123456789012\u{1f642}\nend""#);
    assert_eq!(
        document.nodes[0].arguments,
        ["1234567890123456789012🙂\nend"]
    );
}

#[test]
fn manifest_parser_rejects_types_nonstring_entries_and_duplicate_properties() {
    for source in [
        "(tag)n",
        "n (tag)\"v\"",
        "n p=(tag)\"v\"",
        "n 1",
        "n #true",
        "n #false",
        "n #null",
        "n #inf",
        "n p=1e999999999",
    ] {
        fails(source, ManifestLimits::default(), ManifestErrorKind::Shape);
    }
    for source in ["n p=\"one\" p=\"two\"", r#"n "k"="one" "\u{6b}"="two""#] {
        fails(
            source,
            ManifestLimits::default(),
            ManifestErrorKind::DuplicateProperty,
        );
    }
    // A slash-dashed property is not a declaration and cannot replace the live one.
    assert_eq!(parse("n p=one /- p=two").nodes[0].properties["p"], "one");
}

#[test]
fn manifest_parser_rejects_malformed_and_truncated_documents() {
    for source in [
        "n {",
        "}",
        "n \"unfinished",
        "n p=",
        "n /*",
        "n { c;",
        "n { c; } trailing",
        "n \"\\q\"",
        "n [bad]",
        "n r\"v1 raw\"",
        "n true",
        ")",
        "n (tag",
        "/-",
    ] {
        fails(source, ManifestLimits::default(), ManifestErrorKind::Syntax);
    }
    assert!(parse("").nodes.is_empty());
    assert!(parse("// empty\n/* comment */").nodes.is_empty());
}

#[test]
fn manifest_limits_charge_exact_source_nodes_entries_and_decoded_text() {
    let source = "a x=\"é\" \"b\"; c";
    let exact = ManifestLimits {
        max_source_bytes: source.len(),
        max_nodes: 2,
        max_entries: 2,
        max_depth: 1,
        // a, x, é (two UTF-8 bytes), b, c.
        max_text_bytes: 6,
    };
    assert_eq!(parse_document(source, exact).unwrap().nodes.len(), 2);
    fails(
        source,
        ManifestLimits {
            max_source_bytes: source.len() - 1,
            ..exact
        },
        ManifestErrorKind::SourceBytes,
    );
    fails(
        source,
        ManifestLimits {
            max_nodes: 1,
            ..exact
        },
        ManifestErrorKind::Nodes,
    );
    fails(
        source,
        ManifestLimits {
            max_entries: 1,
            ..exact
        },
        ManifestErrorKind::Entries,
    );
    fails(
        source,
        ManifestLimits {
            max_text_bytes: 5,
            ..exact
        },
        ManifestErrorKind::TextBytes,
    );
    // Decoded bytes, not source escape bytes, determine the retained-text budget.
    let escaped = r#"n "\u{e9}""#;
    assert_eq!(
        parse_document(
            escaped,
            ManifestLimits {
                max_text_bytes: 3,
                ..ManifestLimits::default()
            }
        )
        .unwrap()
        .nodes[0]
            .arguments,
        ["é"]
    );
    fails(
        escaped,
        ManifestLimits {
            max_text_bytes: 2,
            ..ManifestLimits::default()
        },
        ManifestErrorKind::TextBytes,
    );
}

#[test]
fn manifest_depth_limits_cover_live_nodes_and_slash_dashed_child_blocks() {
    let limits = ManifestLimits {
        max_depth: 2,
        ..ManifestLimits::default()
    };
    assert_eq!(parse_document("a { b; }", limits).unwrap().nodes.len(), 2);
    fails("a { b { c; }; }", limits, ManifestErrorKind::Depth);
    fails(
        "/- a { b { c { d; }; }; }\nkept",
        limits,
        ManifestErrorKind::Depth,
    );
    fails(
        "a /- { b { c { d; }; }; }",
        limits,
        ManifestErrorKind::Depth,
    );
    let omitted = parse_document("/- a { b { c; }; }\nkept", limits).unwrap();
    assert_eq!(omitted.nodes.len(), 1);
    assert_eq!(omitted.nodes[0].name, "kept");
    // Braces inside raw/escaped strings or comments are lexical text, not depth.
    assert_eq!(
        parse_document(
            r##"a #"{{{{"# "\u{7b}" /* {{{{ */"##,
            ManifestLimits {
                max_depth: 1,
                ..limits
            }
        )
        .unwrap()
        .nodes
        .len(),
        1
    );
}

#[test]
fn manifest_limits_reject_invalid_configuration_before_parsing() {
    let default = ManifestLimits::default();
    for limits in [
        ManifestLimits {
            max_source_bytes: 0,
            ..default
        },
        ManifestLimits {
            max_nodes: 0,
            ..default
        },
        ManifestLimits {
            max_entries: 0,
            ..default
        },
        ManifestLimits {
            max_depth: 0,
            ..default
        },
        ManifestLimits {
            max_depth: MAX_MANIFEST_DEPTH + 1,
            ..default
        },
        ManifestLimits {
            max_text_bytes: 0,
            ..default
        },
    ] {
        fails("invalid ]", limits, ManifestErrorKind::InvalidLimits);
    }
    fails(
        "invalid ]",
        ManifestLimits {
            max_source_bytes: 1,
            ..default
        },
        ManifestErrorKind::SourceBytes,
    );
}

#[test]
fn manifest_nested_comments_and_rejected_depth_do_not_use_recursive_dom() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let comments = format!("{}{}kept", "/*".repeat(8192), "*/".repeat(8192));
            assert_eq!(parse(&comments).nodes[0].name, "kept");
            let nested = format!("{}leaf;{}", "a { ".repeat(8192), "};".repeat(8192));
            fails(&nested, ManifestLimits::default(), ManifestErrorKind::Depth);
            let skipped = format!("/- {nested}");
            fails(
                &skipped,
                ManifestLimits::default(),
                ManifestErrorKind::Depth,
            );
            let malformed = ")".repeat(65536);
            fails(
                &malformed,
                ManifestLimits::default(),
                ManifestErrorKind::Syntax,
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn manifest_depth_hard_boundary_and_large_single_token_are_checked() {
    let exact = format!(
        "{}leaf;{}",
        "a { ".repeat(MAX_MANIFEST_DEPTH - 1),
        "};".repeat(MAX_MANIFEST_DEPTH - 1)
    );
    let limits = ManifestLimits {
        max_depth: MAX_MANIFEST_DEPTH,
        ..ManifestLimits::default()
    };
    assert_eq!(
        parse_document(&exact, limits).unwrap().nodes.len(),
        MAX_MANIFEST_DEPTH
    );
    fails(
        &exact,
        ManifestLimits {
            max_depth: MAX_MANIFEST_DEPTH - 1,
            ..limits
        },
        ManifestErrorKind::Depth,
    );
    let token = format!("n \"{}\"", "x".repeat(65536));
    fails(
        &token,
        ManifestLimits {
            max_text_bytes: 64,
            ..limits
        },
        ManifestErrorKind::TextBytes,
    );
    let error = parse_document(
        &token,
        ManifestLimits {
            max_source_bytes: token.len() - 1,
            ..limits
        },
    )
    .unwrap_err();
    assert_eq!(error.kind, ManifestErrorKind::SourceBytes);
    assert!(error.to_string().len() < 128);
}
