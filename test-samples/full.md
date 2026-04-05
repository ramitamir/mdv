# Heading 1 — The Main Title

This is a paragraph with **bold text**, *italic text*, and `inline code`. It also has [a link](https://example.com) and some regular text that should wrap nicely across the terminal width.

## Heading 2 — Section Header

### Heading 3 — Subsection

#### Heading 4 — Minor Section

##### Heading 5 — Detail Level

###### Heading 6 — Fine Print

---

## Code Blocks

Here's a Rust code block:

```rust
fn main() {
    let message = "Hello, mdv!";
    println!("{}", message);

    for i in 0..10 {
        if i % 2 == 0 {
            println!("{} is even", i);
        }
    }
}
```

And some Python:

```python
def fibonacci(n):
    a, b = 0, 1
    for _ in range(n):
        yield a
        a, b = b, a + b

for num in fibonacci(10):
    print(num)
```

## Tables

| Feature       | Status  | Priority |
|---------------|---------|----------|
| Headlines     | Done    | High     |
| Code blocks   | Done    | High     |
| Tables        | Done    | Medium   |
| Images        | Planned | Low      |

## Lists

### Unordered

- First item
- Second item with **bold** 🙂 
  - Nested item one
  - Nested item two
    - Deep nested
- Third item

### Ordered

1. Step one
2. Step two
3. Step three
   1. Sub-step A
   2. Sub-step B

## Block Quotes

> This is a block quote. It should be rendered with a colored left border
> and slightly indented from the main text.
>
> — Some Author

> Nested quotes:
>> This is a nested block quote.
>> It goes deeper.

## Inline Styles

This paragraph has **bold**, *italic*, ***bold italic***, ~~strikethrough~~, and `inline code` all mixed together. Here's a longer paragraph to test word wrapping behavior across the terminal width — it should break at word boundaries and maintain readable line lengths without cutting words in half.

## Horizontal Rules

Above the rule.

---

Below the rule.

---

## Images

Here is an inline image:

![Test image](test-image.png)

And a missing image:

![Missing](nonexistent.png)

The end!
