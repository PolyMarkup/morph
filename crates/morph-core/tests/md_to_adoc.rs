use morph::format::Format;
use pretty_assertions::assert_eq;

fn convert(input: &str) -> String {
    morph::convert(input, Format::Markdown, Format::AsciiDoc).unwrap()
}

// === Headings ===

#[test]
fn heading_level_1() {
    assert_eq!(convert("# Title #"), "= Title\n");
}

#[test]
fn heading_level_1_underscored() {
    assert_eq!(convert("Title\n====="), "= Title\n");
}

#[test]
fn heading_level_2() {
    assert_eq!(convert("## Title"), "== Title\n");
}

#[test]
fn heading_level_2_underscored() {
    assert_eq!(convert("Title\n-----"), "== Title\n");
}

#[test]
fn heading_level_3() {
    assert_eq!(convert("### Title"), "=== Title\n");
}

#[test]
fn heading_level_4() {
    assert_eq!(convert("#### Title"), "==== Title\n");
}

#[test]
fn heading_level_5() {
    assert_eq!(convert("##### Title"), "===== Title\n");
}

#[test]
fn heading_level_6() {
    assert_eq!(convert("###### Title"), "====== Title\n");
}

#[test]
fn heading_different_styling() {
    assert_eq!(convert("# Title ####"), "= Title\n");
}

#[test]
fn heading_with_backticks() {
    assert_eq!(convert("### `what.ever.Foo`"), "=== `what.ever.Foo`\n");
}

// === Paragraphs ===

#[test]
fn paragraph_single_line() {
    assert_eq!(
        convert("A paragraph with a single line."),
        "A paragraph with a single line.\n"
    );
}

#[test]
fn paragraph_multiple_lines() {
    assert_eq!(
        convert("First line of paragraph.\nSecond line of paragraph."),
        "First line of paragraph.\nSecond line of paragraph.\n"
    );
}

#[test]
fn paragraph_multiple_paragraphs() {
    assert_eq!(
        convert("First paragraph.\n\nSecond paragraph."),
        "First paragraph.\n\nSecond paragraph.\n"
    );
}

#[test]
fn paragraph_special_characters() {
    assert_eq!(
        convert("This is an example: `Provider<List<File>>`"),
        "This is an example: `Provider<List<File>>`\n"
    );
}

// === Code ===

#[test]
fn code_block_no_language() {
    assert_eq!(
        convert("```\nsummary(cars$dist)\nsummary(cars$speed)\n```"),
        "----\nsummary(cars$dist)\nsummary(cars$speed)\n----\n"
    );
}

#[test]
fn code_block_no_language_with_html() {
    assert_eq!(
        convert(
            "```\nNo language indicated, so no syntax highlighting.\nBut let's throw in a <b>tag</b>.\n```"
        ),
        "----\nNo language indicated, so no syntax highlighting.\nBut let's throw in a <b>tag</b>.\n----\n"
    );
}

#[test]
fn code_block_javascript() {
    assert_eq!(
        convert("```javascript\nvar s = \"JavaScript syntax highlighting\";\nalert(s);\n```"),
        "[source,javascript]\n----\nvar s = \"JavaScript syntax highlighting\";\nalert(s);\n----\n"
    );
}

#[test]
fn code_block_python() {
    assert_eq!(
        convert("```python\ns = \"Python syntax highlighting\"\nprint s\n```"),
        "[source,python]\n----\ns = \"Python syntax highlighting\"\nprint s\n----\n"
    );
}

#[test]
fn code_block_indented() {
    assert_eq!(
        convert("    $ gem install asciidoctor"),
        "----\n$ gem install asciidoctor\n----\n"
    );
}

#[test]
fn code_block_adjacent_to_paragraph() {
    assert_eq!(
        convert(
            "Here's an example:\n```javascript\nvar s = \"JavaScript syntax highlighting\";\nalert(s);\n```"
        ),
        "Here's an example:\n\n[source,javascript]\n----\nvar s = \"JavaScript syntax highlighting\";\nalert(s);\n----\n"
    );
}

#[test]
fn inline_code() {
    assert_eq!(
        convert("We defined the `add` function to"),
        "We defined the `add` function to\n"
    );
}

#[test]
fn inline_code_ellipsis() {
    assert_eq!(
        convert("Use `foo(...)` for that"),
        "Use `+foo(...)+` for that\n"
    );
}

#[test]
fn inline_code_arrow() {
    assert_eq!(convert("The `->` operator"), "The `+->+` operator\n");
}

#[test]
fn inline_code_double_plus() {
    assert_eq!(
        convert("Use `a ++ b` in code"),
        "Use `pass:c[a ++ b]` in code\n"
    );
}

#[test]
fn inline_code_attribute_reference() {
    assert_eq!(convert("Set `{foo}` value"), "Set `+{foo}+` value\n");
}

#[test]
fn inline_code_adjacent_to_text() {
    assert_eq!(
        convert("`RegularFile`s are nice"),
        "``RegularFile``s are nice\n"
    );
}

#[test]
fn inline_code_with_space() {
    assert_eq!(convert("Use `code` here"), "Use `code` here\n");
}

#[test]
fn no_extra_lines_between_code_blocks() {
    let input =
        "foo\n\n```kotlin\nprintln(\"bar\")\n```\n\nbaz\n\n```kotlin\nprintln(\"bam\")\n```";
    let expected = "foo\n\n[source,kotlin]\n----\nprintln(\"bar\")\n----\n\nbaz\n\n[source,kotlin]\n----\nprintln(\"bam\")\n----\n";
    assert_eq!(convert(input), expected);
}

// === Markup ===

#[test]
fn no_formatting() {
    assert_eq!(convert("Normal text"), "Normal text\n");
}

#[test]
fn no_formatting_multiline() {
    assert_eq!(
        convert("Normal text\nNormal text"),
        "Normal text\nNormal text\n"
    );
}

#[test]
fn no_formatting_multi_paragraphs() {
    assert_eq!(
        convert("Normal text\n\nNormal text"),
        "Normal text\n\nNormal text\n"
    );
}

#[test]
fn text_lists_text() {
    let input = "The support provides:\n\n* Understanding of implicit browser methods (e.g. `to()`, `at()`) in test classes (e.g. `extends GebSpec`)\n* Understanding of content defined via the Content DSL (within `Page` and `Module` classes only)\n* Completion in `at {}` and `content {}` blocks\n\nThis effectively enables more authoring support with less explicit type information. The Geb development team would like to thank the good folks at JetBrains for adding this explicit support for Geb to IDEA.";
    let expected = "The support provides:\n\n* Understanding of implicit browser methods (e.g. `to()`, `at()`) in test classes (e.g. `extends GebSpec`)\n* Understanding of content defined via the Content DSL (within `Page` and `Module` classes only)\n* Completion in `at {}` and `content {}` blocks\n\nThis effectively enables more authoring support with less explicit type information. The Geb development team would like to thank the good folks at JetBrains for adding this explicit support for Geb to IDEA.\n";
    assert_eq!(convert(input), expected);
}

#[test]
fn bold_text() {
    assert_eq!(
        convert("**Bold text**\n__Bold text__"),
        "*Bold text*\n*Bold text*\n"
    );
}

#[test]
fn italic_text() {
    assert_eq!(
        convert("*Italic text*\n_Italic text_"),
        "_Italic text_\n_Italic text_\n"
    );
}

#[test]
fn mono_text() {
    assert_eq!(convert("`Mono text`"), "`Mono text`\n");
}

#[test]
fn bold_and_italic() {
    assert_eq!(
        convert("This is ***bold and italic*** text"),
        "This is *_bold and italic_* text\n"
    );
}

#[test]
fn blockquotes() {
    let input = "> Blockquotes are very handy in email to emulate reply text.\n> This line is part of the same quote.\n\nQuote break.\n\n> This is a very long line that will still be quoted properly when it wraps. Oh boy let's keep writing to make sure this is long enough to actually wrap for everyone. Oh, you can *put* **Markdown** into a blockquote.";
    let expected = "____\n\nBlockquotes are very handy in email to emulate reply text.\nThis line is part of the same quote.\n\n____\n\nQuote break.\n\n____\n\nThis is a very long line that will still be quoted properly when it wraps. Oh boy let's keep writing to make sure this is long enough to actually wrap for everyone. Oh, you can _put_ *Markdown* into a blockquote.\n\n____\n";
    assert_eq!(convert(input), expected);
}

#[test]
fn nested_blockquotes() {
    let input = "> > What's new?\n>\n> I've got Markdown in my AsciiDoc!\n>\n> > Like what?\n>\n> * Blockquotes\n> * Headings\n> * Fenced code blocks\n>\n> > Is there more?\n>\n> Yep. AsciiDoc and Markdown share a lot of common syntax already.";
    let expected = "____\n\n________\n\nWhat's new?\n\n________\n\nI've got Markdown in my AsciiDoc!\n\n________\n\nLike what?\n\n________\n\n* Blockquotes\n* Headings\n* Fenced code blocks\n\n________\n\nIs there more?\n\n________\n\nYep. AsciiDoc and Markdown share a lot of common syntax already.\n\n____\n";
    assert_eq!(convert(input), expected);
}

#[test]
fn superscript() {
    assert_eq!(convert("superscript^2^"), "superscript^2^\n");
}

#[test]
fn subscript() {
    assert_eq!(convert("CO~2~"), "CO~2~\n");
}

#[test]
fn double_angle_bracket_quoting() {
    assert_eq!(convert("<<hello>>"), "\u{ab}hello\u{bb}\n");
}

#[test]
fn double_quoting() {
    assert_eq!(convert("\"hello\""), "\"hello\"\n");
}

#[test]
fn single_quoting() {
    assert_eq!(convert("'hello'"), "'hello'\n");
}

#[test]
fn apostrophe() {
    assert_eq!(convert("a'a"), "a'a\n");
}

#[test]
fn ellipsis() {
    assert_eq!(convert("a...a\na. . .a"), "a\u{2026}a\na\u{2026}a\n");
}

#[test]
fn em_dash() {
    assert_eq!(convert("a---a"), "a\u{2014}a\n");
}

#[test]
fn en_dash() {
    assert_eq!(convert("a--a"), "a\u{2013}a\n");
}

#[test]
fn nbsp() {
    assert_eq!(convert("<< a a >>"), "\u{ab}{nbsp}a a{nbsp}\u{bb}\n");
}

#[test]
fn hard_line_break() {
    assert_eq!(
        convert("Roses are red,  \nViolets are blue. \nSort of blue.\nMore like violet."),
        "Roses are red, +\nViolets are blue.\nSort of blue.\nMore like violet.\n"
    );
}

#[test]
fn strikethrough() {
    assert_eq!(
        convert("This is ~~striked~~ text"),
        "This is [line-through]#striked# text\n"
    );
}

// === Lines ===

#[test]
fn four_horizontal_rules() {
    assert_eq!(
        convert("---\n\n- - -\n\n***\n\n* * *"),
        "'''\n'''\n'''\n'''\n"
    );
}

// === Links ===

#[test]
fn implicit_inline_link() {
    assert_eq!(
        convert("Use [http://example.com](http://example.com) for sample links in documentation."),
        "Use http://example.com for sample links in documentation.\n"
    );
}

#[test]
fn inline_link() {
    assert_eq!(
        convert("This is [an example](http://example.com/) inline link."),
        "This is http://example.com/[an example] inline link.\n"
    );
}

#[test]
fn linked_text_with_comma() {
    assert_eq!(
        convert("This is [a very, very cool](http://example.com/) inline link."),
        "This is http://example.com/[\"a very, very cool\"] inline link.\n"
    );
}

#[test]
fn reference_style_link_with_definition() {
    assert_eq!(
        convert(
            "The [syntax page] [s] provides complete, detailed documentation for\n\n[s]: /projects/markdown/syntax  \"Markdown Syntax\""
        ),
        "The link:/projects/markdown/syntax[syntax page] provides complete, detailed documentation for\n"
    );
}

#[test]
fn reference_style_link_with_link_text() {
    assert_eq!(
        convert(
            "The [syntax page] provides complete, detailed documentation for\n\n[syntax page]: http://www.syntaxpage.com"
        ),
        "The http://www.syntaxpage.com[syntax page] provides complete, detailed documentation for\n"
    );
}

#[test]
fn internal_link() {
    assert_eq!(
        convert("Refer to [Quick start](#quick-start) to learn how to get started."),
        "Refer to <<quick-start,Quick start>> to learn how to get started.\n"
    );
}

#[test]
fn reference_style_image() {
    assert_eq!(
        convert("![Alt text][logo]\n\n[logo]: images/icons/home.png"),
        "image:images/icons/home.png[Alt text]\n"
    );
}

#[test]
fn inline_image_with_parameters() {
    assert_eq!(
        convert(
            "![Alt text](images/icons/home.png)\n\n![Alt text](images/icons/home.png?width=100)"
        ),
        "image:images/icons/home.png[Alt text]\n\nimage:images/icons/home.png?width=100[Alt text]\n"
    );
}

#[test]
fn inline_image_with_comma_in_alt() {
    assert_eq!(
        convert("![Alt,text](images/icons/home.png)"),
        "image:images/icons/home.png[\"Alt,text\"]\n"
    );
}

#[test]
fn hyperlinked_inline_image() {
    assert_eq!(
        convert(
            "[![Build Status](https://travis-ci.org/asciidoctor/asciidoctor.png)](https://travis-ci.org/asciidoctor/asciidoctor)"
        ),
        "image:https://travis-ci.org/asciidoctor/asciidoctor.png[Build Status,link=https://travis-ci.org/asciidoctor/asciidoctor]\n"
    );
}

// === Lists ===

#[test]
fn unordered_list() {
    assert_eq!(
        convert("* Item 1\n* Item 2\n* Item 3"),
        "* Item 1\n* Item 2\n* Item 3\n"
    );
}

#[test]
fn unordered_list_of_paragraphs() {
    assert_eq!(
        convert("* Paragraph 1\n\n* Paragraph 2"),
        "* Paragraph 1\n\n* Paragraph 2\n"
    );
}

#[test]
fn unordered_nested_list() {
    assert_eq!(
        convert("* Item 1\n    * Item 1_1\n    * Item 1_2"),
        "* Item 1\n** Item 1_1\n** Item 1_2\n"
    );
}

#[test]
fn ordered_list() {
    assert_eq!(
        convert("1. Item 1\n1. Item 2\n1. Item 3"),
        ". Item 1\n. Item 2\n. Item 3\n"
    );
}

#[test]
fn nested_ordered_list() {
    assert_eq!(
        convert("1. Item 1\n    1. Item 1.1\n    1. Item 1.2"),
        ". Item 1\n.. Item 1.1\n.. Item 1.2\n"
    );
}

#[test]
fn nested_ordered_with_unordered() {
    let input = "1. Item 1\n    1. Item 11\n        * bullet 111\n        * bullet 112\n            * bullet 1121\n                1. Item 11211\n    1. Item 12\n1. Item 2";
    let expected = ". Item 1\n.. Item 11\n*** bullet 111\n*** bullet 112\n**** bullet 1121\n..... Item 11211\n.. Item 12\n. Item 2\n";
    assert_eq!(convert(input), expected);
}

#[test]
fn ordered_with_nested_unordered() {
    let input = "1. Item 1\n\n2. Item 2\n\n    * Subitem of Item 2\n\n3. Item 3";
    let expected = ". Item 1\n\n. Item 2\n\n** Subitem of Item 2\n\n. Item 3\n";
    assert_eq!(convert(input), expected);
}

#[test]
fn ordered_list_of_paragraphs() {
    // Note: the reference test has ". Paragraph 1" as markdown input,
    // which in standard markdown would be parsed as a paragraph starting with ". "
    // not an ordered list. The reference Java tool uses pegdown which may differ.
    // We'll test with standard "1. Paragraph 1" syntax instead.
    assert_eq!(
        convert("1. Paragraph 1\n\n2. Paragraph 2"),
        ". Paragraph 1\n\n. Paragraph 2\n"
    );
}

#[test]
fn unordered_list_with_link() {
    let input = "There is a Maven example project available.\n\n* [http://github.com/geb/geb-example-maven](https://github.com/geb/geb-example-maven)";
    let expected = "There is a Maven example project available.\n\n* https://github.com/geb/geb-example-maven[http://github.com/geb/geb-example-maven]\n";
    assert_eq!(convert(input), expected);
}

#[test]
fn list_item_ending_with_blank() {
    assert_eq!(convert("- "), "*\n");
}

#[test]
fn multi_line_list_item_with_continuation() {
    assert_eq!(
        convert("* foo\n    * bar\n      baz"),
        "* foo\n** bar\nbaz\n"
    );
}

#[test]
fn list_without_separating_blank_line() {
    let input = "This is my typical list without any empty line for separation\n- first item\n- second item\n- third item";
    let expected = "This is my typical list without any empty line for separation\n\n* first item\n* second item\n* third item\n";
    assert_eq!(convert(input), expected);
}

// === Tables ===

#[test]
fn basic_table() {
    let input = "| Name of Column 1 | Name of Column 2|\n| ---------------- | --------------- |\n| Cell in column 1, row 1 | Cell in column 2, row 1|\n| Cell in column 1, row 2 | Cell in column 2, row 2|";
    let expected = "|===\n|Name of Column 1 |Name of Column 2\n\n|Cell in column 1, row 1 |Cell in column 2, row 1\n|Cell in column 1, row 2 |Cell in column 2, row 2\n|===\n";
    assert_eq!(convert(input), expected);
}

#[test]
fn table_under_paragraph() {
    let input = "# Step\n\nLorem Ipsum etc.\n\n- `sample.yaml`: sample file to do some stuff with\n\n| Name of Column 1 | Name of Column 2|\n| ---------------- | --------------- |\n| Cell in column 1, row 1 | Cell in column 2, row 1|\n| Cell in column 1, row 2 | Cell in column 2, row 2|";
    let expected = "= Step\n\nLorem Ipsum etc.\n\n* `sample.yaml`: sample file to do some stuff with\n|===\n|Name of Column 1 |Name of Column 2\n\n|Cell in column 1, row 1 |Cell in column 2, row 1\n|Cell in column 1, row 2 |Cell in column 2, row 2\n|===\n";
    assert_eq!(convert(input), expected);
}

#[test]
fn table_trailing_space() {
    let input = "| Browser | Tablet |  Smartphone |\n| ------- | ------ | ---------- |\n| Safari 5.1+| iPad 2+ |  iOS 6+ |";
    let expected = "|===\n|Browser |Tablet |Smartphone\n\n|Safari 5.1+ |iPad 2+ |iOS 6+\n|===\n";
    assert_eq!(convert(input), expected);
}

#[test]
fn table_with_alignment() {
    let input = "| Tables        | Are           |  Cool|\n| ------------- |:-------------:| ----:|\n| col 3 is      | right-aligned | $1600|\n| col 2 is      | centered      |   $12|\n| zebra stripes | are neat      |    $1|";
    let expected = "[cols=\"<,^,>\"]\n|===\n|Tables |Are |Cool\n\n|col 3 is |right-aligned |$1600\n|col 2 is |centered |$12\n|zebra stripes |are neat |$1\n|===\n";
    assert_eq!(convert(input), expected);
}

#[test]
fn html_table() {
    let input = "Care must be taken with slashes when specifying both the base URL and the relative URL as trailing and leading slashes have significant meaning. The following table illustrates the resolution of different types of URLs.\n\n<table class=\"graybox\" border=\"0\" cellspacing=\"0\" cellpadding=\"5\">\n    <tr><th>Base</th><th>Navigating To</th><th>Result</th></tr>\n    <tr><td>http://myapp.com/</td><td>abc</td><td>http://myapp.com/abc</td></tr>\n    <tr><td>http://myapp.com</td><td>abc</td><td>http://myapp.comabc</td></tr>\n</table>\n\nIt is usually most desirable to define your base urls with trailing slashes and not to use leading slashes on relative URLs.";
    let expected = "Care must be taken with slashes when specifying both the base URL and the relative URL as trailing and leading slashes have significant meaning. The following table illustrates the resolution of different types of URLs.\n\n|===\n|Base |Navigating To |Result\n\n|http://myapp.com/ |abc |http://myapp.com/abc\n|http://myapp.com |abc |http://myapp.comabc\n|===\n\nIt is usually most desirable to define your base urls with trailing slashes and not to use leading slashes on relative URLs.\n";
    assert_eq!(convert(input), expected);
}

// === Description Lists ===

#[test]
fn description_list() {
    assert_eq!(
        convert("Apple\n:   Pomaceous fruit of plants of the genus Malus in\nthe family Rosaceae."),
        "Apple::\n  Pomaceous fruit of plants of the genus Malus in\n  the family Rosaceae.\n"
    );
}

#[test]
fn multiple_description_lists() {
    let input = "Apple\n:   Pomaceous fruit of plants of the genus Malus in\nthe family Rosaceae.\n\nOrange\n:   The fruit of an evergreen tree of the genus Citrus.";
    let expected = "Apple::\n  Pomaceous fruit of plants of the genus Malus in\n  the family Rosaceae.\nOrange::\n  The fruit of an evergreen tree of the genus Citrus.\n";
    assert_eq!(convert(input), expected);
}
