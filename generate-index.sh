#!/bin/bash

# Script to generate index.html from template with development flag injection
set -euo pipefail

# Check if template exists
if [ ! -f "index.template.html" ]; then
    echo "Error: index.template.html not found" >&2
    exit 1
fi

# Check if we're in release mode (Trunk sets this environment variable)
if [ "${TRUNK_BUILD_RELEASE:-false}" = "true" ]; then
    DEV_FLAG="false"
    echo "Generating index.html for RELEASE build (dev_flag = $DEV_FLAG)"
else
    DEV_FLAG="true"
    echo "Generating index.html for DEBUG build (dev_flag = $DEV_FLAG)"
fi

# Generate index.html from template
if ! sed "s/{dev_flag}/$DEV_FLAG/g" index.template.html > index.html; then
    echo "Error: Failed to generate index.html from template" >&2
    exit 1
fi

# Verify the generated file exists and is not empty
if [ ! -s "index.html" ]; then
    echo "Error: Generated index.html is empty or does not exist" >&2
    exit 1
fi

echo "Successfully generated index.html with dev_flag = $DEV_FLAG"
