= Morph: one document, ten forms

This strict HTML specimen exercises *bold*, _italic_, *_bold italic_*, #strike[strikethrough], superscript E=mc#super[2], subscript H#sub[2]O, `inline_code()`, and a #link("https://github.com/PolyMarkup/morph")[titled link].

A soft line break follows this line
and the paragraph continues here. \
A hard line break starts this sentence on a new visual line.

A linked image is represented inline: #image("../assets/morph-mark.svg"). A tagged raw inline follows: {{ portable_value }}.

== Blocks and structure

```rust
fn main() {
    println!("one AST, many syntaxes");
}
```

#quote[
  === A quotation can contain structure
  
  Blocks remain blocks, even when they are nested.
  
  - Quoted list item one
  - Quoted list item two
]

== Lists

- An unordered item with *markup*
- An item containing a nested ordered list
  + The numbering starts at three
  + The next nested item

+ A top-level ordered item
+ Another ordered item

== Terms and definitions

/ *Morph*: A dependency-free markup converter.A shared document model with multiple emitters.
/ Lossless: Equivalent structure survives a round trip when both formats can express it.

== Alignment and spans

#table(
  columns: 4,
  [Feature],
  [Status],
  [Count],
  [Notes],
  table.cell(rowspan: 2)[Tables],
  [Stable],
  [10],
  [All formats emit a table],
  table.cell(colspan: 2)[This cell spans two columns],
  [Row span at left],
)

#line(length: 100%)

== Native passthrough

<native key="value">
  preserved verbatim
</native>
