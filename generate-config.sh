#!/bin/bash

# Script to generate config.js with development flags
set -euo pipefail

# Create build directory if it doesn't exist
mkdir -p build

echo "Generating build/config.js..."

# Check if we're in release mode - trunk sets TRUNK_PROFILE instead of TRUNK_BUILD_RELEASE
if [ "${TRUNK_PROFILE:-debug}" = "release" ]; then
    echo "RELEASE build detected - generating config.js with dev_flag = false"
    cat > build/config.js << 'EOF'
// Build configuration
window.dev_flag = false;
EOF
else
    echo "DEBUG build detected - generating config.js with dev_flag = true"
    cat > build/config.js << 'EOF'
// Build configuration
window.dev_flag = true;
EOF
fi

echo "Successfully generated build/config.js"
