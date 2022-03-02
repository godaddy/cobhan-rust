#!/bin/sh

if [ "${DEBUG:-0}" -eq "1" ]; then
    BUILD_FLAGS="--features cobhan_debug"
else
    BUILD_FLAGS="--release"
fi

if [ "${ALPINE:-0}" -eq "1" ]; then
    BUILD_FLAGS="${BUILD_FLAGS} -C target-feature=-crt-static"
fi

# Build
cargo build --verbose "${BUILD_FLAGS}" --target-dir target/
