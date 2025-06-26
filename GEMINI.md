- Be direct and honest in your responses.
- Use rust edition 2024
- Don't change unrelated code.
- Don't fix preexisting warnings if they are unrelated to your current changes.

### Code Style
- Don't use `modulename/mod.rs` to define a module, instead use `modulename.rs`
- Use `debug_assert` to check any assumptions you make.
- Prefer panicking over logging of severe or unrecoverable errors.
- If you ignore errors using things like `let _ = ...` write a comment with a good reason for doing so.
- Match exhaustively whenever possible.
- Prefer pull-based solutions over imperative approaches when feasible.

### Building
- Build the entire project using `trunk build`.
- The project is built using `trunk`. Don't add extra shell scripts to compile things.
- Don't add new trunk configurations to subcrates. The top one is the only one allowed.
- The `audio-worklet` crate should produce a binary. Not a library. Trunk expects it that way.

### Testing
- `./test.sh` - Run all tests (ALWAYS use this instead of `cargo test`)
- `./test.sh test_name` - Run specific tests
- `./test.sh --release` - Run tests with additional cargo flags

**CRITICAL**: Always use `./test.sh` for testing. This script auto-detects the correct host target and runs tests with the native target instead of WASM, which is required for proper test execution.
