Dissonance Lab is a single page web application implemented using rust that shows note interval dissonance graphically.
It also allows you to play and hear the notes using a built in piano-like synthesizer.

# Code structure
The project it split into multiple crates. All in the same cargo workspace.
- The main `dissonance-lab` crate in the root.
- `audio-worklet` containing code that will compile to a separate wasm binary and loaded as a WebAudio AudioWorklet.
- `shared-types` containing types shared between the other crates.

# Code Style
- Don't use `modulename/mod.rs` to define a module, instead use `modulename.rs`
- Use `debug_assert` to check any assumptions you make.
- Prefer panicking over logging of severe or unrecoverable errors.
- Don't try to handle logic or programmer errors. These should result in a panic.
- If you ignore errors using things like `let _ = ...` write a comment with a good reason for doing so.
- Match exhaustively whenever possible.
- When formatting strings, prefer inline variable interpolation `format!("{variable}")` over positional arguments `format!("{}", variable)` for better readability and maintainability.
- Strive for a clear and predictable data flow. When designing component interactions, prefer architectures where state is polled from a central source (pull-based) over complex, deeply nested callback chains (push-based), unless the reactive, event-driven nature of the UI demands it.
- Avoid using magic numbers in the code. Instead, define a `const` for such values, placing it as close as possible to where it is used. If a constant is only referenced in one location, keeping it nearby improves code readability by eliminating the need to scroll to find its value. However, do not define a `const` if the value is already clearly documented elsewhere, such as when it appears in a `match` statement for an enum.
- Avoid unsafe. If you really think you need unsafe, ask the user first, and write a detailed comment why unsafe was required.
- Avoid wildcard imports (`use x::*;`) unless explicitly recommended for a specific case, such as importing a crate's prelude. Prefer listing only the items you need to improve code clarity and maintainability.
- Place comments on the line above the code they reference, rather than as trailing comments on the same line.
- If you decide to solve a warning by using `#[allow(...)]`, write a comment about why you think it is ok. And think a second time about whether it really is ok..

# Conventions
- Use rust edition 2024
- We use `egui` as our GUI library.
- In `egui`, the coordinate system has the x-axis increasing to the right and the y-axis increasing downward.

# Quality Assurance
- **MANDATORY**: Before completing any task, run `cargo xtask check`.
- If this command fails or show NEW warnings/errors compared to before your changes, you MUST fix them
- Pre-existing warnings unrelated to your changes should be left alone
- Document any intentional ignoring of errors with detailed comments explaining why
- If you need to skip formatting checks during development, use `cargo xtask check --skip-fmt`

# Running
- For development the project is started using `cargo xtask dev`. But the agent shouldn't call that. The user will keep that running continuously.
- For mobile testing, use `cargo xtask dev --bind 0.0.0.0` to serve on all network interfaces, then access via your local IP address (e.g., `http://192.168.1.100:8080`)
- **CRITICAL**: Before using `cargo xtask dump-latest-logs` to check audio-related functionality or any runtime behavior, you MUST first ask the user to "unmute" or "click to enable audio". Browsers block audio until a user interaction (like a click). This means the audio worklet and related Rust code will not execute, and any runtime errors in that code will not appear in the console until after the user has clicked on the page.
- **MANDATORY WORKFLOW**: When working with audio-related code or checking for runtime errors:
  1. FIRST: Ask user to unmute/click the page to enable audio
  2. THEN: Use `cargo xtask dump-latest-logs` to read the frontend logs
  3. Do NOT skip step 1 - the logs will be stale/incomplete without user interaction
- Use `cargo xtask dump-latest-logs` to read the frontend logs of the most recent session.

# Temporary Tools
- If you need to create temporary scripts, tools, or files for debugging, analysis, or one-time tasks, place them in `tmp/` directory at the project root.
- When creating temporary scripts or tools, set them up as separate cargo projects instead of standalone `.rs` files.
- Clean up temporary files when they are no longer needed.

