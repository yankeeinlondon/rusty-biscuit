**Broken references in [root.md](<root>/root.md)**

**[root.md](<root>/root.md)**

|  |  |  |
| :-- | :-- | :-- |
| <span style="color: rgb(251, 44, 54)">file not found</span> _in hyperlink_ | `./missing-one.md` | [root.md:9:5](<root>/root.md#intro-root '{ qp: { line: 9, column: 5 } }') |
| <span style="color: rgb(251, 44, 54)">file not found</span> _in hyperlink_ | `./missing-two.md` | [root.md:15:9](<root>/root.md#second '{ qp: { line: 15, column: 9 } }') |
| <span style="color: rgb(251, 44, 54)">file not found</span> _in hyperlink_ | `./missing-three.md` | [root.md:22:13](<root>/root.md#second '{ qp: { line: 22, column: 13 } }') |
| <span style="color: rgb(251, 44, 54)">file not found</span> _in hyperlink_ | `./missing-five.md` | [root.md:24:8](<root>/root.md#second '{ qp: { line: 24, column: 8 } }') |

**[child.md](<root>/child.md)** (transcluded at root.md:26)

|  |  |  |
| :-- | :-- | :-- |
| <span style="color: rgb(251, 44, 54)">file not found</span> _in hyperlink_ | `./missing-four.md` | [child.md:7:43](<root>/child.md#child-heading '{ qp: { line: 7, column: 43 } }') |
| <span style="color: rgb(240, 177, 0)">file not found</span> _in hyperlink_ | `./missing-ins.md` | [child.md:9:7](<root>/child.md#child-heading '{ qp: { line: 9, column: 7 } }') |

Checked 10 references across 2 documents: 5 ok, <span style="color: rgb(251, 44, 54)">**5 broken**</span>

1 warning
