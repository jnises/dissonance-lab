# Development Commands

This project uses `cargo xtask` for simplified development workflows.

## Quick Start

To start the complete development environment, run:

```bash
cargo xtask dev
```

This single command will:
- Start the backend HTTP log server.
- Start the Trunk development server.
- Enable automatic builds with hot-reloading.
- Forward frontend console logs to your terminal.

When you're done, simply press `Enter` or `Ctrl+C` in the terminal to shut everything down gracefully.

## Available Commands

### `cargo xtask dev`

Starts the all-in-one development server.

- **Frontend**: `http://localhost:8080`
- **Log Server**: `http://localhost:3001`

### `cargo xtask dump-log`

Dumps the log messages from the most recent development session.

- The command finds the last `New session started` marker in `tmp/dev-log-server.log` and prints everything after it.
- This is useful for debugging issues that occurred during the last time you ran `cargo xtask dev`.

## Manual Setup (Alternative)

If you prefer to run the components manually:

1.  **Start the log server** (in one terminal):
    ```bash
    cargo run -p dev-log-server
    ```

2.  **Start Trunk** (in another terminal):
    ```bash
    trunk serve
    ```

## Build Commands

For standalone builds, use Trunk directly:

```bash
# Build for development (with debug logging)
trunk build

# Build for release (optimized, no debug logging)
trunk build --release
```