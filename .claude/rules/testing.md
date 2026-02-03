---
version: 0.2.0
updated: 2026-02-03
---

# Rust Testing

## Tools

- **Built-in test framework** - `cargo test`
- **cargo-watch** - Watch mode during TDD: `cargo watch -x test`

---

## Test Organization

**Unit tests: separate files via module declaration.**

This project keeps tests in their own files to avoid bloating source modules. The mechanism is standard Rust module resolution — `mod tests;` (with semicolon) tells the compiler to find the module in a file, and `#[cfg(test)]` ensures it's only compiled during `cargo test`.

```
src/
  scanner.rs          ← implementation + one line at bottom: #[cfg(test)] mod tests;
  scanner/
    tests.rs          ← unit tests for scanner
  parser.rs
  parser/
    tests.rs
```

In the source file (`scanner.rs`), the only test-related line:
```rust
#[cfg(test)]
mod tests;
```

In the test file (`scanner/tests.rs`), access private items the same way as inline tests:
```rust
use super::*;

#[test]
fn test_something() {
    assert_eq!(do_thing(), expected);
}
```

**Integration tests:** In `tests/` directory at project root. Can only access the public API.

```rust
// tests/integration_test.rs
use claude_tracker::public_api;

#[test]
fn test_integration() {
    let result = public_api::process_data();
    assert!(result.is_ok());
}
```

---

## Testing Guidelines

**Rust-specific:**
- Use `assert!`, `assert_eq!`, `assert_ne!` for assertions
- Use `#[should_panic]` for tests expecting panics
- Use `Result<()>` return type for tests that can fail with `?`
