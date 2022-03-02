#!/bin/bash
set -e

# If DEBUG is set, add debug- to the file names
if [ "${DEBUG:-0}" -eq "1" ]; then
    DEBUG_FN_PART="debug-"
else
    DEBUG_FN_PART=""
fi

# Normalize machine architecture for file names
case $(uname -m) in
"x86_64")
    SYS_FN_PART="x64"
    ;;
"aarch64")
    SYS_FN_PART="arm64"
    ;;
"arm64")
    SYS_FN_PART="arm64"
    ;;
*)
    echo "Unknown machine $(uname -m)!"
    exit 255
    ;;
esac

# If ALPINE is set, include musl in the file name
if [ "${ALPINE:-0}" -eq "1" ]; then
    SYS_FN_PART="musl-${SYS_FN_PART}"
fi

# OS Detection
case $(uname -s) in
"Darwin")
    IS_MACOS=1
    DYN_EXT=".dylib"
    RUST_EXT="-darwin.rlib"
    A_EXT="-darwin.a"
    ;;
"Linux")
    IS_MACOS=0
    DYN_EXT=".so"
    RUST_EXT=".rlib"
    A_EXT=".a"
    ;;
*)
    echo "Unknown system $(uname -s)!"
    exit 255
    ;;
esac
export IS_MACOS
export DYN_EXT
export RUST_EXT
export A_EXT

# Create dynamic library file name suffix
DYN_SUFFIX="${DEBUG_FN_PART}${SYS_FN_PART}${DYN_EXT}"
export DYN_SUFFIX

# Create Rust static library file name suffix
RLIB_SUFFIX="${DEBUG_FN_PART}${SYS_FN_PART}${RUST_EXT}"
export RLIB_SUFFIX

if [ "${DEBUG:-0}" -eq "1" ]; then
    BUILD_FLAGS="--features cobhan_debug"
    BUILD_DIR="debug"
else
    BUILD_FLAGS="--release"
    BUILD_DIR="release"
fi

if [ "${ALPINE:-0}" -eq "1" ]; then
    RUSTFLAGS="-C target-feature=-crt-static"
    export RUSTFLAGS
fi

# Build
echo "Compiling (Rust) ${BUILD_DIR}/libcobhandemo${DYN_EXT}"
cargo build --verbose ${BUILD_FLAGS} --target-dir target/

# Test Rust dynamic library file
count=0
while [ $count -lt 20 ]; do
    echo "Test iteration ${count}"
    python3 test/consumer_console_app.py "target/${BUILD_DIR}/libcobhandemo${DYN_EXT}"
    if [ "$?" -eq "0" ]; then
        echo "Passed"
    else
        echo "Tests failed (Rust): libcobhandemo-${DYN_SUFFIX} Result: $?"
        exit 255
    fi
    count=$(expr ${count} + 1)
done

echo "Tests passed (Rust): libcobhandemo-${DYN_SUFFIX}"

# Create output directory
mkdir -p ./output/

# Copy Rust dynamic library file
cp "target/${BUILD_DIR}/libcobhandemo${DYN_EXT}" "output/libcobhandemo-${DYN_SUFFIX}"

# Copy Rust static library file
cp "target/${BUILD_DIR}/libcobhandemo.rlib" "output/libcobhandemo-${RLIB_SUFFIX}"
