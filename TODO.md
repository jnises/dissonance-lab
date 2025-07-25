- [x] Don't show the "shift for multi select" label when showing the gui on a phone, or when the screen is too narrow for it to fit comfortably

## Review Comments from PR #7

- [x] Rename `MIN_WIDTH_FOR_LABEL` to a more descriptive name like `MOBILE_SCREEN_WIDTH_THRESHOLD` or `MOBILE_BREAKPOINT_WIDTH` (src/app.rs, line ~186)
- [x] Use `Array.prototype.includes()` instead of `indexOf` for better readability in cache whitelist check (assets/sw.js, line ~38)
- [x] Use `const` instead of `var` for `cacheWhitelist` since it's not reassigned (assets/sw.js, line ~29)
- [x] Extract magic number 480.0 to a module-level constant with descriptive name for better maintainability (src/app.rs, line ~186)

## Review Comments from PR #8

- [x] Extract magic number 0.2 to a named constant to improve code maintainability and readability (src/app.rs)
- [x] Extract magic numbers -1.0 and 6.0 to named constants to improve code maintainability and readability (src/app.rs)
- [x] Extract magic number 2.0 to named constants to improve code maintainability and readability (src/app.rs)
- [x] Extract magic number 10.0 to a named constant to improve code maintainability and readability (src/app.rs)
- [x] Extract magic number 0.8 to a named constant to improve code maintainability and readability (src/app.rs)
- [x] Extract magic number 0.5 to a named constant to improve code maintainability and readability (src/app.rs)
- [x] Extract magic number 1.5 to a named constant to improve code maintainability and readability (src/app.rs)
- [x] Extract magic numbers -4.0 and 4.0 to named constants to improve code maintainability and readability (src/app.rs)
- [x] Update comment to describe 'rotated text' instead of 'vertical text' for accuracy (src/app.rs)

- [x] Add some graphic that hints the user that they should press the "mute" button to enable audio
- [ ] Disable the pwa caching when running in debug mode
- [ ] Go through the codebase and make sure the guidelines from the instructions file are applied.
- [ ] Add all todos in the code as subtasks here.
- [ ] Change the order of the interval displays so the bottom row shows the first pressed note when using the mouse, and the actual base when using a midi keyboard.
