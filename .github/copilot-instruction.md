Dissonance Lab is a single page web application implemented using rust that shows note interval dissonance graphically.
It also allows you to play and hear the notes using a built in piano-like synthesizer.

### General instructions
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
- We use `trunk` for building the project.
- Build the entire project using `trunk build`.

### Testing
- `./test.sh` - Run all tests (ALWAYS use this instead of `cargo test`)
- `./test.sh test_name` - Run specific tests
- `./test.sh --release` - Run tests with additional cargo flags

**CRITICAL**: Always use `./test.sh` for testing. This script auto-detects the correct host target and runs tests with the native target instead of WASM, which is required for proper test execution.

### Running
- Run the project using `trunk serve`.
- There currently is no way to pipe browser console logs back to the terminal, so for now you have to ask me to tell you what the console says.
- Browsers do not allow you to play audio without the user first interacting with the page. This means the user needs to click to unmute the page before any of the audio code is actually run.
