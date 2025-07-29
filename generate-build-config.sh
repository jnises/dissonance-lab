#!/bin/bash

# Script to generate build-config.js with development flags and log forwarding functionality
#
# ⚠️  WARNING: If you modify this script, you need to restart 'trunk serve' to pick up the changes!
#     The generated file is cached and changes won't be reflected until restart.
#
set -euo pipefail

# Create build directory if it doesn't exist
mkdir -p build

echo "Generating build/build-config.js..."

# Check if we're in release mode - trunk sets TRUNK_PROFILE instead of TRUNK_BUILD_RELEASE
if [ "${TRUNK_PROFILE:-debug}" = "release" ]; then
    echo "RELEASE build detected - generating build-config.js WITHOUT log forwarding"
    cat > build/build-config.js << 'EOF'
// Build configuration
window.dev_flag = false;
// No log forwarding code in release builds
EOF
else
    echo "DEBUG build detected - generating build-config.js WITH log forwarding"
    cat > build/build-config.js << 'EOF'
// Build configuration
window.dev_flag = true;

// Development Log Forwarding - only included in debug builds
(function() {
    // Store original console methods
    const originalConsole = {
        log: console.log,
        warn: console.warn,
        error: console.error,
        debug: console.debug,
        info: console.info
    };
    
    // Log buffer for batching
    let logBuffer = [];
    let flushTimer = null;
    
    // Function to flush logs to server
    function flushLogs() {
        if (logBuffer.length === 0) return;
        
        const logsToSend = logBuffer.splice(0); // Clear buffer
        
        // Send each log entry
        logsToSend.forEach(logEntry => {
            fetch('/logs', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(logEntry)
            }).catch(error => {
                // Silently ignore errors to avoid infinite loops
                // Could add fallback to originalConsole.error here if needed
            });
        });
    }
    
    // Function to schedule flush
    function scheduleFlush() {
        if (flushTimer) return;
        flushTimer = setTimeout(() => {
            flushTimer = null;
            flushLogs();
        }, 100); // Flush every 100ms
    }
    
    // Function to create intercepted console method
    function createInterceptor(level, originalMethod) {
        return function(...args) {
            // Call original method first
            originalMethod.apply(console, args);
            
            // Format message and clean up CSS styling from console_log crate
            let message;
            let logLevel = level;
            let file = null;
            let line = null;
            
            if (args.length > 0 && typeof args[0] === 'string' && args[0].includes('%c')) {
                // This is a styled console message from console_log crate
                // The first argument is the format string with %c markers
                // Subsequent arguments are CSS styles that we want to ignore
                message = args[0];
                
                // Remove all %c markers and their associated CSS
                message = message.replace(/%c/g, '');
                
                // Extract log level from the message if present (console_log crate includes it)
                const levelMatch = message.match(/^(ERROR|WARN|INFO|DEBUG|TRACE)\s+/);
                if (levelMatch) {
                    logLevel = levelMatch[1].toLowerCase();
                    // Remove the level prefix from the message
                    message = message.substring(levelMatch[0].length);
                }
            } else {
                // Regular console message - process all arguments
                message = args.map(arg => {
                    if (typeof arg === 'string') return arg;
                    if (arg instanceof Error) return arg.toString();
                    try {
                        return JSON.stringify(arg);
                    } catch {
                        return String(arg);
                    }
                }).join(' ');
            }
            
            // Extract file and line information from the final message (regardless of source)
            // Format can be: "src/file.rs:123: message" or "/full/path/to/file.rs:123 message"
            const fileLineMatch = message.match(/^([^:\s]+):(\d+):?\s*(.*)/);
            if (fileLineMatch) {
                file = fileLineMatch[1];
                line = parseInt(fileLineMatch[2], 10);
                message = fileLineMatch[3];
            }
            
            // Clean up extra whitespace
            message = message.replace(/\s+/g, ' ').trim();
            
            // Add to buffer
            logBuffer.push({
                level: logLevel,
                message: message,
                file: file,
                line: line
            });
            
            // Flush immediately if buffer is large, or schedule flush
            if (logBuffer.length >= 10) {
                flushLogs();
            } else {
                scheduleFlush();
            }
        };
    }
    
    // Intercept console methods
    console.log = createInterceptor('info', originalConsole.log);
    console.info = createInterceptor('info', originalConsole.info);
    console.warn = createInterceptor('warn', originalConsole.warn);
    console.error = createInterceptor('error', originalConsole.error);
    console.debug = createInterceptor('debug', originalConsole.debug);
    
    // Flush any remaining logs when page unloads
    window.addEventListener('beforeunload', flushLogs);
    
    console.log('=== DISSONANCE_LAB_SESSION_START ===');
    console.log('Development log forwarding enabled');
})();
EOF
fi

echo "Successfully generated build/build-config.js"
