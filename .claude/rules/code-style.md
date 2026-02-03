---
version: 0.1.0
updated: 2026-01-30
---

# Rust Code Style

## Philosophy

**Write idiomatic Rust. Let the compiler and type system guide you.**

Rust's design enforces most good patterns automatically. Use `rustfmt` and `clippy` as your primary style guides.

---

## !! CRITICAL !! Anti-Patterns to Avoid

### Overusing `.clone()`

**Don't clone to fight the borrow checker.**

```rust
// â Avoid - cloning to satisfy borrow checker
fn process_data(data: &Data) -> ProcessedData {
    let cloned = data.clone();  // Unnecessary clone
    expensive_operation(cloned)
}

// â Good - work with references or redesign ownership
fn process_data(data: &Data) -> ProcessedData {
    expensive_operation(data)  // Borrow instead
}
```

**If you're cloning frequently, reconsider your design.**
- Can you borrow instead?
- Should the function take ownership?
- Is `Rc<T>` or `Arc<T>` more appropriate?

### Overusing `.unwrap()`

**Don't panic in library code or on expected errors.**

```rust
// â Avoid - panics on error
fn load_config(path: &str) -> Config {
    std::fs::read_to_string(path)
        .unwrap()  // Panics if file doesn't exist
}

// â Good - return Result
fn load_config(path: &str) -> Result<Config> {
    let contents = std::fs::read_to_string(path)?;
    Ok(parse_config(&contents)?)
}
```

**Use `.unwrap()` only when:**
- In tests
- You have proven invariants (document why)
- Early prototyping (replace with proper error handling later)

---

## Error Handling

**Use `anyhow` for application error handling:**

```rust
use anyhow::{Context, Result};

fn process_file(path: &str) -> Result<Data> {
    let contents = std::fs::read_to_string(path)
        .context("Failed to read file")?;

    parse_data(&contents)
        .context("Failed to parse data")
}
```

---

## Tooling

```bash
cargo fmt      # Format code (run before commits)
cargo clippy   # Lint and catch common mistakes
```

Follow clippy's suggestions unless you have good reason not to.
