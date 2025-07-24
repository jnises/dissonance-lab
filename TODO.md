- [x] Don't show the "shift for multi select" label when showing the gui on a phone, or when the screen is too narrow for it to fit comfortably

## Review Comments from PR #7

- [x] Rename `MIN_WIDTH_FOR_LABEL` to a more descriptive name like `MOBILE_SCREEN_WIDTH_THRESHOLD` or `MOBILE_BREAKPOINT_WIDTH` (src/app.rs, line ~186)
- [ ] Use `Array.prototype.includes()` instead of `indexOf` for better readability in cache whitelist check (assets/sw.js, line ~38)
- [ ] Use `const` instead of `var` for `cacheWhitelist` since it's not reassigned (assets/sw.js, line ~29)
- [ ] Extract magic number 480.0 to a module-level constant with descriptive name for better maintainability (src/app.rs, line ~186)


- [ ] Add some graphic that hints the user that they should press the "mute" button to enable audio
