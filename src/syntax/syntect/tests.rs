use super::*;

fn highlight(path: &str, lines: &[&str]) -> Vec<Vec<SyntaxSpan>> {
    crate::syntax::default_highlighter()
        .highlight(
            Path::new(path),
            &lines.iter().map(|line| (*line).into()).collect::<Vec<_>>(),
        )
        .unwrap()
}

fn category_at(line: &str, needle: &str, spans: &[SyntaxSpan]) -> SyntaxCategory {
    let offset = line.find(needle).unwrap();
    spans
        .iter()
        .find(|span| span.range.contains(&offset))
        .unwrap()
        .category
}

#[test]
fn json_keys_strings_numbers_and_booleans_have_semantic_categories() {
    let line = r#"{"name": "wolf", "count": 42, "enabled": true}"#;
    let spans = highlight("settings.JSON", &[line]);

    assert_eq!(
        category_at(line, "name", &spans[0]),
        SyntaxCategory::Property
    );
    assert_eq!(category_at(line, "wolf", &spans[0]), SyntaxCategory::String);
    assert_eq!(category_at(line, "42", &spans[0]), SyntaxCategory::Number);
    assert_eq!(
        category_at(line, "true", &spans[0]),
        SyntaxCategory::Boolean
    );
}

#[test]
fn multiline_comments_preserve_state_and_unicode_byte_boundaries() {
    let lines = [
        "/* wolf",
        "ещё comment */ int count = 42;",
        "const char *s = \"狼\";",
    ];
    let output = highlight("example.cpp", &lines);

    assert_eq!(
        category_at(lines[1], "ещё", &output[1]),
        SyntaxCategory::Comment
    );
    assert_eq!(
        category_at(lines[1], "int", &output[1]),
        SyntaxCategory::Type
    );
    assert_eq!(
        category_at(lines[2], "狼", &output[2]),
        SyntaxCategory::String
    );

    for (line, spans) in lines.iter().zip(output) {
        let reconstructed: String = spans.iter().map(|span| &line[span.range.clone()]).collect();
        assert_eq!(reconstructed, *line);
    }
}

#[test]
fn common_languages_and_extensionless_shebangs_are_detected() {
    for (path, code, needle, category) in [
        (
            "main.rs",
            "fn main() { let x = 42; }",
            "main",
            SyntaxCategory::Function,
        ),
        (
            "main.py",
            "def greet(): return 'hi'",
            "greet",
            SyntaxCategory::Function,
        ),
        ("init.lua", "local count = 42", "42", SyntaxCategory::Number),
        ("config.yaml", "count: 42", "42", SyntaxCategory::Number),
        ("README.md", "# Heading", "Heading", SyntaxCategory::Heading),
    ] {
        let spans = highlight(path, &[code]);
        assert!(!spans.is_empty(), "missing syntax for {path}");
        assert_eq!(category_at(code, needle, &spans[0]), category, "{path}");
    }

    let lines = ["#!/usr/bin/env python3", "def greet(): pass"];
    let spans = highlight("script", &lines);
    assert_eq!(
        category_at(lines[1], "greet", &spans[1]),
        SyntaxCategory::Function
    );
}

#[test]
fn unknown_formats_and_work_limits_keep_plain_text_available() {
    assert!(highlight("file.unknown-format", &["just text"]).is_empty());

    let long = "x".repeat(4097);
    assert!(highlight("file.rs", &[&long, "let x = 1;"]).is_empty());

    let lines = vec!["let x = 1;".to_owned(); 1025];
    assert_eq!(
        crate::syntax::default_highlighter()
            .highlight(Path::new("file.rs"), &lines)
            .unwrap()
            .len(),
        1024
    );
}
