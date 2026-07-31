Morph: one document, ten forms
==============================

This strict HTML specimen exercises **bold**, *italic*, ***bold italic***, ~~strikethrough~~, superscript E=mc\ :sup:`2`\ , subscript H\ :sub:`2`\ O, ``inline_code()``, and a `titled link <https://github.com/PolyMarkup/morph>`_.

A soft line break follows this line
and the paragraph continues here.
| A hard line break starts this sentence on a new visual line.

A linked image is represented inline: |Morph mark|. A tagged raw inline follows: {{ portable_value }}.

Blocks and structure
--------------------

.. code-block:: rust

   fn main() {
       println!("one AST, many syntaxes");
   }

   A quotation can contain structure
   ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

   Blocks remain blocks, even when they are nested.

   * Quoted list item one
   * Quoted list item two

Lists
-----

* An unordered item with **markup**
* An item containing a nested ordered list
  #. The numbering starts at three
  #. The next nested item

#. A top-level ordered item
#. Another ordered item

Terms and definitions
---------------------

**Morph**
   A dependency-free markup converter.
   A shared document model with multiple emitters.

Lossless
   Equivalent structure survives a round trip when both formats can express it.

Alignment and spans
-------------------

+---------+---------------------+-------+--------------------------+
| Feature | Status              | Count | Notes                    |
+=========+=====================+=======+==========================+
| Tables  | Stable              | 10    | All formats emit a table |
+         +---------------------+-------+--------------------------+
|         | This cell spans two columns | Row span at left         |
+---------+---------------------+-------+--------------------------+

----

Native passthrough
------------------

<native key="value">
  preserved verbatim
</native>
