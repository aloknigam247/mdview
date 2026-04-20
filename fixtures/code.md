# Code highlighting

Inline: the function `fn main()` prints to stdout.

## Rust

```rust
use std::collections::BTreeMap;

fn main() {
    let mut m: BTreeMap<&str, i32> = BTreeMap::new();
    m.insert("curvy", 42);
    for (k, v) in &m {
        println!("╭ {k} = {v} ╮");
    }
}
```

## TypeScript

```ts
interface Theme {
  name: string;
  radii: { sm: number; md: number; lg: number };
}

export const dark: Theme = {
  name: "dark",
  radii: { sm: 6, md: 10, lg: 16 },
};
```

## Python

```python
def greet(name: str) -> str:
    return f"hello, {name}"

if __name__ == "__main__":
    print(greet("world"))
```

## Bash

```bash
#!/usr/bin/env bash
set -euo pipefail
for f in fixtures/*.md; do
  echo "╭── $f ──╮"
  wc -l "$f"
done
```

## JSON

```json
{
  "name": "mdview",
  "version": "0.1.0",
  "radii": { "sm": 6, "md": 10, "lg": 16 }
}
```

## Indented code (4-space)

    let x = 1;
    let y = x + 1;

## No-language fence

```
plain preformatted text
with ╭╮╰╯ borders drawn by hand
```
