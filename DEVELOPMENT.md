# Development Commands

This project uses standard Rust and Trunk commands for development.

## Development Environment

To start the complete development environment:

1. **Start the log server** (in one terminal):
   ```bash
   cargo run -p dev-log-server --target aarch64-apple-darwin
   ```

2. **Start Trunk development server** (in another terminal):
   ```bash
   trunk serve
   ```

This will start:
- HTTP log server on port 3001
- Trunk development server on port 8080
- Automatic builds with hot reload
- Log forwarding from frontend to backend terminal

## Build Commands

```bash
# Build for development (with debug logging)
trunk build

# Build for release (optimized, no debug logging)
trunk build --release
```

## Development Features

### Frontend Log Forwarding
- **Development builds**: Console logs are automatically forwarded to the backend log server
- **Release builds**: Log forwarding code is removed by minification for performance
- Uses `window.dev_flag` from generated `build/config.js`

### Pre-build Hooks
The project uses pre-build hooks that run automatically:
- `build-audio-worklet.sh` - Builds the audio worklet WASM module  
- `generate-config.sh` - Creates `build/config.js` with `window.dev_flag` based on build mode

### Hot Reload
Trunk will automatically rebuild and reload when you change:
- Rust source files
- HTML template
- JavaScript files
- Assets

## URLs
- **Frontend**: http://localhost:8080
- **Log Server**: http://localhost:3001

## Testing Log Forwarding

You can test the log forwarding manually:
```bash
curl -X POST 'http://localhost:8080/logs' \
  -H 'Content-Type: application/json' \
  -d '{"level":"info","message":"test from curl"}'
```
