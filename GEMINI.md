Be direct and honest in your responses.

### Code Style
- Don't use `modulename/mod.rs` to define a module, instead use `modulename.rs`
- Use `debug_assert` to check any assumptions you make.
- Prefer panicking over logging of severe or unrecoverable errors.
- Prefer exhaustive matching to any form of wildcards.
- Prefer pull-based solutions over imperative approaches when feasible.

### Testing
- `./test.sh` - Run all tests (ALWAYS use this instead of `cargo test`)
- `./test.sh test_name` - Run specific tests
- `./test.sh --release` - Run tests with additional cargo flags

**CRITICAL**: Always use `./test.sh` for testing. This script auto-detects the correct host target and runs tests with the native target instead of WASM, which is required for proper test execution.

### Building
- The project is built using `trunk`. Don't add extra shell scripts to compile things.
- Don't add new trunk configurations to subcrates. The top one is the only one allowed.
- The `audio-worklet` crate should produce a binary. Not a library. Trunk expects it that way.
