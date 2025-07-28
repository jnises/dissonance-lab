# Development Commands

This project uses `xtask` for development utilities. The xtask binary must be built for your native platform (not WASM).

## Easiest Option: Auto-Detecting Script

The project includes a `./dev` script that automatically detects your platform:

```bash
# Start development environment
./dev dev

# Build for release  
./dev build

# Build for debug
./dev build-debug

# Show help
./dev --help
```

## Direct Commands

```bash
# Start development environment (log server + trunk serve)
cargo run -p xtask --target aarch64-apple-darwin dev    # Apple Silicon Mac
cargo run -p xtask --target x86_64-apple-darwin dev     # Intel Mac  
cargo run -p xtask --target x86_64-unknown-linux-gnu dev   # Linux

# Build for release
cargo run -p xtask --target aarch64-apple-darwin build

# Build for debug  
cargo run -p xtask --target aarch64-apple-darwin build-debug
```

## Recommended Aliases

Add these to your shell configuration file (`~/.zshrc`, `~/.bashrc`, etc.):

### For Apple Silicon Macs (M1/M2/M3):
```bash
# Dissonance Lab development aliases
alias ddev='cargo run -p xtask --target aarch64-apple-darwin dev'
alias dbuild='cargo run -p xtask --target aarch64-apple-darwin build'
alias dbuild-debug='cargo run -p xtask --target aarch64-apple-darwin build-debug'
```

### For Intel Macs:
```bash
# Dissonance Lab development aliases  
alias ddev='cargo run -p xtask --target x86_64-apple-darwin dev'
alias dbuild='cargo run -p xtask --target x86_64-apple-darwin build'
alias dbuild-debug='cargo run -p xtask --target x86_64-apple-darwin build-debug'
```

### For Linux:
```bash
# Dissonance Lab development aliases
alias ddev='cargo run -p xtask --target x86_64-unknown-linux-gnu dev'
alias dbuild='cargo run -p xtask --target x86_64-unknown-linux-gnu build'  
alias dbuild-debug='cargo run -p xtask --target x86_64-unknown-linux-gnu build-debug'
```

After adding the aliases, reload your shell or run `source ~/.zshrc` (or your shell config file).

## Development Environment

The development environment (`./dev dev` or the aliases) starts:
- HTTP log server on port 3001
- Trunk development server on port 8080
- Automatic index.html generation
- Audio worklet building

Press Enter or Ctrl+C to stop the development environment.

## What Changed

The xtask implementation replaces the trunk pre-build hooks with proper process management:
- `generate-index.sh` is now called before trunk starts (solving the chicken-and-egg problem)
- Both log server and trunk serve are managed as child processes
- Proper cleanup on shutdown
- Consistent build process for both development and release modes

## Why the Target Flag is Needed

The main workspace is configured to build for WASM by default (in `.cargo/config.toml`), but the xtask development utility needs to run natively. The `--target` flag ensures xtask builds for your native platform instead of WASM. The `./dev` script handles this automatically.
