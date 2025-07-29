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
            
            // Format message
            const message = args.map(arg => {
                if (typeof arg === 'string') return arg;
                if (arg instanceof Error) return arg.toString();
                try {
                    return JSON.stringify(arg);
                } catch {
                    return String(arg);
                }
            }).join(' ');
            
            // Add to buffer
            logBuffer.push({
                level: level,
                message: message,
                target: 'frontend',
                timestamp: new Date().toISOString()
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
    
    console.log('New session started');
    console.log('Development log forwarding enabled');
})();
EOF
fi

echo "Successfully generated build/build-config.js"
