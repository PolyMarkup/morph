use morph::format::Format;

/// Helper: convert input through two formats and compare the ASTs
fn roundtrip_ast(input: &str, from: Format, through: Format) {
    let doc1 = morph::parse(input, from).expect("parse original");
    let intermediate = morph::emit(&doc1, through).expect("emit intermediate");
    let doc2 = morph::parse(&intermediate, through).expect("parse intermediate");
    let output = morph::emit(&doc2, from).expect("emit back");
    let doc3 = morph::parse(&output, from).expect("parse roundtripped");

    // Compare ASTs (doc1 vs doc3)
    assert_eq!(
        doc1, doc3,
        "\n--- Roundtrip AST mismatch ---\nOriginal ({:?}):\n{}\nIntermediate ({:?}):\n{}\nRoundtripped ({:?}):\n{}\n",
        from, input, through, intermediate, from, output
    );
}

// --- Markdown -> AsciiDoc -> Markdown roundtrips ---

#[test]
fn roundtrip_md_adoc_heading() {
    roundtrip_ast("# Hello world\n", Format::Markdown, Format::AsciiDoc);
}

#[test]
fn roundtrip_md_adoc_bold() {
    roundtrip_ast("**bold text**\n", Format::Markdown, Format::AsciiDoc);
}

#[test]
fn roundtrip_md_adoc_italic() {
    roundtrip_ast("*italic text*\n", Format::Markdown, Format::AsciiDoc);
}

#[test]
fn roundtrip_md_adoc_code_block() {
    roundtrip_ast(
        "```python\nprint('hello')\n```\n",
        Format::Markdown,
        Format::AsciiDoc,
    );
}

#[test]
fn roundtrip_md_adoc_link() {
    roundtrip_ast(
        "[click here](https://example.com)\n",
        Format::Markdown,
        Format::AsciiDoc,
    );
}

#[test]
fn roundtrip_md_adoc_unordered_list() {
    roundtrip_ast(
        "- item 1\n- item 2\n- item 3\n",
        Format::Markdown,
        Format::AsciiDoc,
    );
}

#[test]
fn roundtrip_md_adoc_ordered_list() {
    roundtrip_ast(
        "1. first\n2. second\n3. third\n",
        Format::Markdown,
        Format::AsciiDoc,
    );
}

#[test]
fn roundtrip_md_adoc_hr() {
    roundtrip_ast(
        "text above\n\n---\n\ntext below\n",
        Format::Markdown,
        Format::AsciiDoc,
    );
}

// --- Markdown -> RST -> Markdown roundtrips ---

#[test]
fn roundtrip_md_rst_heading() {
    roundtrip_ast("# Hello world\n", Format::Markdown, Format::Rst);
}

#[test]
fn roundtrip_md_rst_bold() {
    roundtrip_ast("**bold text**\n", Format::Markdown, Format::Rst);
}

#[test]
fn roundtrip_md_rst_italic() {
    roundtrip_ast("*italic text*\n", Format::Markdown, Format::Rst);
}

#[test]
fn roundtrip_md_rst_code_block() {
    roundtrip_ast(
        "```python\nprint('hello')\n```\n",
        Format::Markdown,
        Format::Rst,
    );
}

#[test]
fn roundtrip_md_rst_unordered_list() {
    roundtrip_ast(
        "- item 1\n- item 2\n- item 3\n",
        Format::Markdown,
        Format::Rst,
    );
}

// --- Markdown -> Typst -> Markdown roundtrips ---

#[test]
fn roundtrip_md_typst_heading() {
    roundtrip_ast("# Hello world\n", Format::Markdown, Format::Typst);
}

#[test]
fn roundtrip_md_typst_bold() {
    roundtrip_ast("**bold text**\n", Format::Markdown, Format::Typst);
}

#[test]
fn roundtrip_md_typst_italic() {
    roundtrip_ast("*italic text*\n", Format::Markdown, Format::Typst);
}

#[test]
fn roundtrip_md_typst_code_block() {
    roundtrip_ast(
        "```python\nprint('hello')\n```\n",
        Format::Markdown,
        Format::Typst,
    );
}

#[test]
fn roundtrip_md_typst_unordered_list() {
    roundtrip_ast(
        "- item 1\n- item 2\n- item 3\n",
        Format::Markdown,
        Format::Typst,
    );
}

// --- Nested list roundtrips ---

#[test]
fn roundtrip_md_adoc_nested_unordered_list() {
    roundtrip_ast(
        "- A\n  - B\n  - C\n- D\n",
        Format::Markdown,
        Format::AsciiDoc,
    );
}

#[test]
fn roundtrip_md_adoc_nested_ordered_list() {
    roundtrip_ast(
        "1. A\n   1. B\n   2. C\n2. D\n",
        Format::Markdown,
        Format::AsciiDoc,
    );
}

#[test]
fn roundtrip_md_rst_nested_unordered_list() {
    roundtrip_ast("- A\n  - B\n  - C\n- D\n", Format::Markdown, Format::Rst);
}

#[test]
fn roundtrip_md_rst_nested_ordered_list() {
    roundtrip_ast(
        "1. A\n   1. B\n   2. C\n2. D\n",
        Format::Markdown,
        Format::Rst,
    );
}

#[test]
fn roundtrip_md_typst_nested_unordered_list() {
    roundtrip_ast("- A\n  - B\n  - C\n- D\n", Format::Markdown, Format::Typst);
}

#[test]
fn roundtrip_md_typst_nested_ordered_list() {
    roundtrip_ast(
        "1. A\n   1. B\n   2. C\n2. D\n",
        Format::Markdown,
        Format::Typst,
    );
}

// --- AsciiDoc -> Markdown -> AsciiDoc roundtrips ---

#[test]
fn roundtrip_adoc_md_heading() {
    roundtrip_ast("= Hello world\n", Format::AsciiDoc, Format::Markdown);
}

#[test]
fn roundtrip_adoc_md_bold() {
    roundtrip_ast("*bold text*\n", Format::AsciiDoc, Format::Markdown);
}

#[test]
fn roundtrip_adoc_md_italic() {
    roundtrip_ast("_italic text_\n", Format::AsciiDoc, Format::Markdown);
}

// --- Direct format tests ---

#[test]
fn md_to_rst_heading() {
    let result = morph::convert("# Hello\n", Format::Markdown, Format::Rst).unwrap();
    assert_eq!(result, "Hello\n=====\n");
}

#[test]
fn md_to_rst_bold() {
    let result = morph::convert("**bold**\n", Format::Markdown, Format::Rst).unwrap();
    assert_eq!(result, "**bold**\n");
}

#[test]
fn md_to_rst_italic() {
    let result = morph::convert("*italic*\n", Format::Markdown, Format::Rst).unwrap();
    assert_eq!(result, "*italic*\n");
}

#[test]
fn md_to_rst_code_block() {
    let result = morph::convert(
        "```python\nprint('hello')\n```\n",
        Format::Markdown,
        Format::Rst,
    )
    .unwrap();
    assert!(result.contains(".. code-block:: python"));
    assert!(result.contains("   print('hello')"));
}

#[test]
fn md_to_rst_link() {
    let result = morph::convert(
        "[click](https://example.com)\n",
        Format::Markdown,
        Format::Rst,
    )
    .unwrap();
    assert!(result.contains("`click <https://example.com>`_"));
}

#[test]
fn md_to_rst_unordered_list() {
    let result = morph::convert("- item 1\n- item 2\n", Format::Markdown, Format::Rst).unwrap();
    assert!(result.contains("* item 1"));
    assert!(result.contains("* item 2"));
}

#[test]
fn md_to_rst_ordered_list() {
    let result = morph::convert("1. first\n2. second\n", Format::Markdown, Format::Rst).unwrap();
    assert!(result.contains("#. first"));
    assert!(result.contains("#. second"));
}

#[test]
fn md_to_typst_heading() {
    let result = morph::convert("# Hello\n", Format::Markdown, Format::Typst).unwrap();
    assert_eq!(result, "= Hello\n");
}

#[test]
fn md_to_typst_bold() {
    let result = morph::convert("**bold**\n", Format::Markdown, Format::Typst).unwrap();
    assert_eq!(result, "*bold*\n");
}

#[test]
fn md_to_typst_italic() {
    let result = morph::convert("*italic*\n", Format::Markdown, Format::Typst).unwrap();
    assert_eq!(result, "_italic_\n");
}

#[test]
fn md_to_typst_code_block() {
    let result = morph::convert(
        "```python\nprint('hello')\n```\n",
        Format::Markdown,
        Format::Typst,
    )
    .unwrap();
    assert!(result.contains("```python"));
    assert!(result.contains("print('hello')"));
}

#[test]
fn md_to_typst_link() {
    let result = morph::convert(
        "[click](https://example.com)\n",
        Format::Markdown,
        Format::Typst,
    )
    .unwrap();
    assert!(result.contains("#link(\"https://example.com\")[click]"));
}

#[test]
fn md_to_typst_unordered_list() {
    let result = morph::convert("- item 1\n- item 2\n", Format::Markdown, Format::Typst).unwrap();
    assert!(result.contains("- item 1"));
    assert!(result.contains("- item 2"));
}

#[test]
fn md_to_typst_ordered_list() {
    let result = morph::convert("1. first\n2. second\n", Format::Markdown, Format::Typst).unwrap();
    assert!(result.contains("+ first"));
    assert!(result.contains("+ second"));
}

#[test]
fn md_to_typst_strikethrough() {
    let result = morph::convert("~~deleted~~\n", Format::Markdown, Format::Typst).unwrap();
    assert!(result.contains("#strike[deleted]"));
}

#[test]
fn md_to_typst_hr() {
    let result = morph::convert("---\n", Format::Markdown, Format::Typst).unwrap();
    assert!(result.contains("#line(length: 100%)"));
}

#[test]
fn md_to_typst_inline_code() {
    let result = morph::convert("`code`\n", Format::Markdown, Format::Typst).unwrap();
    assert_eq!(result, "`code`\n");
}

#[test]
fn md_to_typst_blockquote() {
    let result = morph::convert("> quoted text\n", Format::Markdown, Format::Typst).unwrap();
    assert!(result.contains("#quote["));
    assert!(result.contains("quoted text"));
}

// --- RST -> Markdown tests ---

#[test]
fn rst_to_md_heading() {
    let result = morph::convert("Hello\n=====\n", Format::Rst, Format::Markdown).unwrap();
    assert_eq!(result, "# Hello\n");
}

#[test]
fn rst_to_md_bold() {
    let result = morph::convert("**bold**\n", Format::Rst, Format::Markdown).unwrap();
    assert_eq!(result, "**bold**\n");
}

#[test]
fn rst_to_md_italic() {
    let result = morph::convert("*italic*\n", Format::Rst, Format::Markdown).unwrap();
    assert_eq!(result, "*italic*\n");
}

#[test]
fn rst_to_md_code_block() {
    let result = morph::convert(
        ".. code-block:: python\n\n   print('hello')\n",
        Format::Rst,
        Format::Markdown,
    )
    .unwrap();
    assert!(result.contains("```python"));
    assert!(result.contains("print('hello')"));
}

#[test]
fn rst_to_md_inline_code() {
    let result = morph::convert("``code``\n", Format::Rst, Format::Markdown).unwrap();
    assert_eq!(result, "`code`\n");
}

#[test]
fn rst_to_md_link() {
    let result = morph::convert(
        "`click <https://example.com>`_\n",
        Format::Rst,
        Format::Markdown,
    )
    .unwrap();
    assert!(result.contains("[click](https://example.com)"));
}

// --- Typst -> Markdown tests ---

#[test]
fn typst_to_md_heading() {
    let result = morph::convert("= Hello\n", Format::Typst, Format::Markdown).unwrap();
    assert_eq!(result, "# Hello\n");
}

#[test]
fn typst_to_md_bold() {
    let result = morph::convert("*bold*\n", Format::Typst, Format::Markdown).unwrap();
    assert_eq!(result, "**bold**\n");
}

#[test]
fn typst_to_md_italic() {
    let result = morph::convert("_italic_\n", Format::Typst, Format::Markdown).unwrap();
    assert_eq!(result, "*italic*\n");
}

#[test]
fn typst_to_md_code_block() {
    let result = morph::convert(
        "```python\nprint('hello')\n```\n",
        Format::Typst,
        Format::Markdown,
    )
    .unwrap();
    assert!(result.contains("```python"));
    assert!(result.contains("print('hello')"));
}

#[test]
fn typst_to_md_link() {
    let result = morph::convert(
        "#link(\"https://example.com\")[click]\n",
        Format::Typst,
        Format::Markdown,
    )
    .unwrap();
    assert!(result.contains("[click](https://example.com)"));
}

#[test]
fn typst_to_md_strikethrough() {
    let result = morph::convert("#strike[deleted]\n", Format::Typst, Format::Markdown).unwrap();
    assert!(result.contains("~~deleted~~"));
}

// --- Cross-format conversions ---

#[test]
fn adoc_to_rst_heading() {
    let result = morph::convert("= Hello\n", Format::AsciiDoc, Format::Rst).unwrap();
    assert!(result.contains("Hello"));
    assert!(result.contains("====="));
}

#[test]
fn adoc_to_typst_heading() {
    let result = morph::convert("= Hello\n", Format::AsciiDoc, Format::Typst).unwrap();
    assert_eq!(result, "= Hello\n");
}

#[test]
fn rst_to_adoc_heading() {
    let result = morph::convert("Hello\n=====\n", Format::Rst, Format::AsciiDoc).unwrap();
    assert_eq!(result, "= Hello\n");
}

#[test]
fn rst_to_typst_heading() {
    let result = morph::convert("Hello\n=====\n", Format::Rst, Format::Typst).unwrap();
    assert_eq!(result, "= Hello\n");
}

#[test]
fn typst_to_adoc_heading() {
    let result = morph::convert("= Hello\n", Format::Typst, Format::AsciiDoc).unwrap();
    assert_eq!(result, "= Hello\n");
}

#[test]
fn typst_to_rst_heading() {
    let result = morph::convert("= Hello\n", Format::Typst, Format::Rst).unwrap();
    assert!(result.contains("Hello"));
    assert!(result.contains("====="));
}
