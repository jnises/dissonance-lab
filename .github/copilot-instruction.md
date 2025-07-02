Dissonance Lab is a single page web application implemented using rust that shows note interval dissonance graphically.
It also allows you to play and hear the notes using a built in piano-like synthesizer.

### General instructions
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
- When formatting strings, prefer inline variable interpolation `format!("{variable}")` over positional arguments `format!("{}", variable)` for better readability and maintainability.
- Strive for a clear and predictable data flow. When designing component interactions, prefer architectures where state is polled from a central source (pull-based) over complex, deeply nested callback chains (push-based), unless the reactive, event-driven nature of the UI demands it.

### Building
- Build the project using `trunk build`.

### Testing
- Run tests using the `./test.sh` script.
  - `./test.sh` - Run all tests.
  - `./test.sh test_name` - Run specific tests.
  - `./test.sh --release` - Run tests with cargo release flags.
- **CRITICAL**: Always use `./test.sh` instead of `cargo test`. The script ensures tests run against the native target, not WASM.

### Running
- Run the project using `trunk serve`.
- Browser console logs are not piped to the terminal. You will have to ask me to check them.
- **IMPORTANT**: Browsers block audio until a user interaction (like a click). This means the audio worklet and related Rust code will not execute, and any runtime errors in that code will not appear in the console until after the user has clicked on the page.