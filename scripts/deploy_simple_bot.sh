#!/bin/bash

# The command `set -e` makes the script exit immediately if any command exits with a non-zero status.
set -e

# Source environment variables from .env file if it exists
if [ -f ".env" ]; then
  # Sourcing .env safely
  export $(grep -v '^#' .env | xargs)
else
  echo "Error: .env file not found. Please copy .env.example to .env and fill in your details."
  exit 1
fi

# Check required variables
if [ -z "$DEPLOY_USER" ] || [ -z "$DEPLOY_HOST" ] || [ -z "$DEPLOY_PATH" ] || [ -z "$TELOXIDE_TOKEN" ]; then
  echo "Error: DEPLOY_USER, DEPLOY_HOST, DEPLOY_PATH, and TELOXIDE_TOKEN must be set in .env"
  exit 1
fi

# Default build target if not specified
BUILD_TARGET=${BUILD_TARGET:-x86_64-unknown-linux-gnu}

# Determine the build directory based on CARGO_BUILD_FLAGS
if [[ "$CARGO_BUILD_FLAGS" == *"--release"* ]]; then
  BUILD_DIR="release"
else
  BUILD_DIR="debug"
fi

echo "Building simple_bot for target $BUILD_TARGET ($BUILD_DIR)..."
# Build the example
# shellcheck disable=SC2086
cargo build --example simple_bot --target "$BUILD_TARGET" $CARGO_BUILD_FLAGS

BINARY_PATH="target/$BUILD_TARGET/$BUILD_DIR/examples/simple_bot"

if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    echo "Make sure the build target is correct and installed (rustup target add ...)"
    exit 1
fi

echo "Deploying to ${DEPLOY_USER}@${DEPLOY_HOST}:${DEPLOY_PATH}..."

# Create directory if not exists
ssh ${DEPLOY_USER}@${DEPLOY_HOST} "mkdir -p ${DEPLOY_PATH}"

# Stop existing process if any (simple kill by name - safe for this example)
echo "Stopping existing simple_bot processes..."
ssh ${DEPLOY_USER}@${DEPLOY_HOST} "pkill -x simple_bot || true"

# Copy binary
echo "Copying binary..."
scp "$BINARY_PATH" "${DEPLOY_USER}@${DEPLOY_HOST}:${DEPLOY_PATH}/simple_bot"

echo "Starting simple_bot..."
echo "Press Ctrl+C to stop watching logs (bot will continue running if detached via nohup, but here we run interactively to see logs)"
echo "----------------------------------------"

# Run the bot with the token
ssh -tt ${DEPLOY_USER}@${DEPLOY_HOST} "cd ${DEPLOY_PATH} && TELOXIDE_TOKEN='${TELOXIDE_TOKEN}' RUST_LOG=debug ./simple_bot"
