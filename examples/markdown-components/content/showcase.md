# Main Heading

Welcome to the **Maudit Custom Components** showcase! This example demonstrates _all_ the available markdown components with custom styling.

## Text Formatting

Here's some **bold text** and _italic text_ and `inline code`. You can also use ~~strikethrough text~~ for corrections.

Let's also test a hard break here:
This line comes after a hard break.

## Links and Images

Here are different types of links:

- [Internal link](/about)
- [External link](https://github.com/maudit-org/maudit)
- [Link with title](https://example.com "Example Website")

![Example image](https://placehold.co/600x300 "A placeholder image")

## Lists

### Unordered List

- First item
- Second item with **bold text**
- Third item with _italic text_
- Fourth item with `inline code`

### Ordered List

1. First numbered item
2. Second numbered item
3. Third numbered item with [an external link](https://example.com)

### Task List

- [x] Completed task
- [ ] Incomplete task
- [x] Another completed task
- [ ] Another incomplete task

## Blockquotes

Here's a regular blockquote:

> This is a regular blockquote with some text that spans multiple lines and contains **bold** and _italic_ text.

And here are GitHub-style blockquotes:

> [!NOTE]
> This is a note blockquote with important information.

> [!TIP]
> This is a tip blockquote with helpful advice.

> [!IMPORTANT]
> This is an important blockquote that you should pay attention to.

> [!WARNING]
> This is a warning blockquote about potential issues.

> [!CAUTION]
> This is a caution blockquote about dangerous operations.

## Code Blocks

Here's some inline `code` and here's a code block:

```rs
fn main() {
    println!("Hello, world!");
    let numbers = vec![1, 2, 3, 4, 5];
    for number in numbers {
        println!("Number: {}", number);
    }
}
```

In Maudit 0.3.0, code blocks do not unfortunately allow for custom components.

## Tables

| Header 1     |             Header 2             |        Header 3 |
| ------------ | :------------------------------: | --------------: |
| Left aligned |          Center aligned          |   Right aligned |
| Data 1       |              Data 2              |          Data 3 |
| More data    |          **Bold data**           |   _Italic data_ |
| `Code data`  | [Link data](https://example.com) | ~~Strike data~~ |

---

## Horizontal Rule

The line above is a horizontal rule that separates sections.

## Nested Elements

Here's a complex example with nested formatting:

1. **First item** with _italic_ and `code`
2. Second item with [an external link](https://example.com) and ~~strikethrough~~
   - Nested unordered item
   - Another nested item with **bold** text
3. Third item with an image: ![Small image](https://placehold.co/100x100)

### Complex Blockquote

> This blockquote contains a list with:
>
> - **Bold text**
> - _Italic text_
> - `Inline code`
> - [A link](https://example.com)

## All Together

This section combines **all** the _components_ in `one` ~~paragraph~~ to show how they work together with [links](https://example.com) and hard breaks:
New line after break!

> [!TIP]
> You can combine all these components to create rich, interactive markdown content with custom styling!
