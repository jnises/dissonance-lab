- Be direct and honest in your responses.
- Use rust edition 2024
- Don't change unrelated code.
- Don't fix preexisting warnings if they are unrelated to your current changes.

### Code Style
- Don't use `modulename/mod.rs` to define a module, instead use `modulename.rs`
- Use `debug_assert` to check any assumptions you make.
- Prefer panicking over logging of severe or unrecoverable errors.
- Don't try to handle logic or programmer errors. These should result in a panic.
- If you ignore errors using things like `let _ = ...` write a comment with a good reason for doing so.
- Match exhaustively whenever possible.
- Prefer pull-based solutions over imperative approaches when feasible.
- When formatting strings, prefer inline variable interpolation `format!("{variable}")` over positional arguments `format!("{}", variable)` for better readability and maintainability.

### Building
- Build the entire project using `trunk build`.

### Testing
- `./test.sh` - Run all tests (ALWAYS use this instead of `cargo test`)
- `./test.sh test_name` - Run specific tests
- `./test.sh --release` - Run tests with additional cargo flags

**CRITICAL**: Always use `./test.sh` for testing. This script auto-detects the correct host target and runs tests with the native target instead of WASM, which is required for proper test execution.
