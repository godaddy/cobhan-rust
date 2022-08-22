//! # cobhan-rust - FFI Data Interface
//!
//! Cobhan FFI is a system for enabling shared code to be written in Rust and
//! consumed from all major languages/platforms in a safe and effective way,
//! using easy helper functions to manage any unsafe data marshaling.
//!
//! ## Types
//!
//! * Supported types
//!     * i32 - 32bit signed integer
//!     * i64 - 64bit signed integer
//!     * fl64 - double precision 64bit IEEE 754 floating point
//!     * Cobhan buffer - length delimited 8bit buffer (no null delimiters)
//!         * utf-8 encoded string
//!         * JSON
//!         * binary data
//! * Cobhan buffer details
//!     * Callers provide the output buffer allocation and capacity
//!     * Called functions can transparently return larger values via temporary files
//!     * **Modern [tmpfs](https://en.wikipedia.org/wiki/Tmpfs) is entirely memory backed**
//! * Return values
//!     * Functions that return scalar values can return the value directly
//!         * Functions *can* use special case and return maximum positive or maximum negative or zero values to
//!             represent error or overflow conditions
//!         * Functions *can* allow scalar values to wrap
//!         * Functions should document their overflow / underflow behavior

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::os::raw::c_char;
use std::ptr::copy_nonoverlapping;
use std::slice::from_raw_parts;
use std::str;

use serde_json::Value;
use tempfile::NamedTempFile;

/// No Error
pub const ERR_NONE: i32 = 0;

/// One of the provided pointers is NULL / nil / 0
pub const ERR_NULL_PTR: i32 = -1;

/// One of the provided buffer lengths is too large
pub const ERR_BUFFER_TOO_LARGE: i32 = -2;

/// One of the provided buffers was too small
pub const ERR_BUFFER_TOO_SMALL: i32 = -3;

/// Failed to copy a buffer (copy length != expected length)
pub const ERR_COPY_FAILED: i32 = -4;

/// Failed to decode a JSON buffer
pub const ERR_JSON_DECODE_FAILED: i32 = -5;

/// Failed to encode to JSON buffer
pub const ERR_JSON_ENCODE_FAILED: i32 = -6;

/// UTF8 in a String or JSON is invalid.
pub const ERR_INVALID_UTF8: i32 = -7;

/// TempFile for large partial data failed to read.
pub const ERR_READ_TEMP_FILE_FAILED: i32 = -8;

/// TempFile for large partial data failed to write.
pub const ERR_WRITE_TEMP_FILE_FAILED: i32 = -9;

/// 64 bit buffer header provides 8 byte alignment for data pointers
pub const BUFFER_HEADER_SIZE: isize = 64 / 8;

const SIZEOF_INT32: isize = 32 / 8;

#[cfg(feature = "cobhan_debug")]
macro_rules! debug_print {
    ($( $args:expr ),*) => { println!($($args ),*); };
}

#[cfg(not(feature = "cobhan_debug"))]
macro_rules! debug_print {
    ($( $args:expr ),*) => {};
}

/// Takes a pointer to an external Cobhan Buffer and fallibly attempts to interpret it as a `Vec<u8>`.
///
/// ## Notes
///
/// This function does a memcopy from the provided Cobhan Buffer into Rust owned data.
///
/// ## Safety
///
/// Behavior is undefined if any of the following conditions are violated:
/// - The Cobhan Buffer Header size is not correctly reserved or formatted.
/// - Any of the Safety conditions of [`std::slice::from_raw_parts`][] is violated.
pub unsafe fn cbuffer_to_vector(buffer: *const c_char) -> Result<Vec<u8>, i32> {
    if buffer.is_null() {
        debug_print!("cbuffer_to_vector: buffer is NULL");
        return Err(ERR_NULL_PTR);
    }
    let length = *(buffer as *const i32);
    let _reserved = buffer.offset(SIZEOF_INT32) as *const i32;
    let payload = buffer.offset(BUFFER_HEADER_SIZE) as *const u8;
    debug_print!("cbuffer_to_vector: raw length field is {}", length);

    if length < 0 {
        debug_print!("cbuffer_to_vector: calling temp_to_vector");
        return temp_to_vector(payload, length);
    }

    //Allocation: to_vec() is a clone/copy
    Ok(from_raw_parts(payload, length as usize).to_vec())
}

/// Takes a pointer to an external Cobhan Buffer and fallibly attempts to interpret it as a `String`.
///
/// The String is fallibly checked to ensure UTF-8 formatting.
///
/// ## Notes
///
/// This function does a memcopy from the provided Cobhan Buffer into Rust owned data.
///
/// ## Safety
///
/// Behavior is undefined if any of the following conditions are violated:
/// - The Cobhan Buffer Header size is not correctly reserved or formatted.
/// - Any of the Safety conditions of [`std::slice::from_raw_parts`][] is violated.
pub unsafe fn cbuffer_to_string(buffer: *const c_char) -> Result<String, i32> {
    if buffer.is_null() {
        debug_print!("cbuffer_to_string: buffer is NULL");
        return Err(ERR_NULL_PTR);
    }
    let length = *(buffer as *const i32);
    let _reserved = buffer.offset(SIZEOF_INT32) as *const i32;
    let payload = buffer.offset(BUFFER_HEADER_SIZE) as *const u8;
    debug_print!("cbuffer_to_string: raw length field is {}", length);

    debug_print!("cbuffer_to_string: raw length field is {}", length);

    if length < 0 {
        debug_print!("cbuffer_to_string: calling temp_to_string");
        return temp_to_string(payload, length);
    }

    str::from_utf8(from_raw_parts(payload, length as usize))
        .map(|s| s.to_owned())
        .map_err(|_| {
            debug_print!(
                "cbuffer_to_string: payload is invalid utf-8 string (length = {})",
                length
            );
            ERR_INVALID_UTF8
        })
}

/// Gets a tempfile data for a payload and interprets it as a `String`.
unsafe fn temp_to_string(payload: *const u8, length: i32) -> Result<String, i32> {
    let file_name =
        str::from_utf8(from_raw_parts(payload, (0 - length) as usize)).map_err(|_| {
            debug_print!(
                "temp_to_string: temp file name is invalid utf-8 string (length = {})",
                0 - length
            );
            ERR_INVALID_UTF8
        })?;

    debug_print!("temp_to_string: reading temp file {}", file_name);

    fs::read_to_string(file_name).map_err(|_e| {
        debug_print!(
            "temp_to_string: Error reading temp file {}: {}",
            file_name,
            _e
        );
        ERR_READ_TEMP_FILE_FAILED
    })
}

/// Gets a tempfile data for a payload and interprets it as a `Vec<u8>`.
unsafe fn temp_to_vector(payload: *const u8, length: i32) -> Result<Vec<u8>, i32> {
    let file_name =
        str::from_utf8(from_raw_parts(payload, (0 - length) as usize)).map_err(|_| {
            debug_print!(
                "temp_to_vector: temp file name is invalid utf-8 string (length = {})",
                0 - length
            );
            ERR_INVALID_UTF8
        })?;

    fs::read(file_name).map_err(|_e| {
        debug_print!(
            "temp_to_vector: failed to read temporary file {}: {}",
            file_name,
            _e
        );
        ERR_READ_TEMP_FILE_FAILED
    })
}

/// Takes a pointer to an external Cobhan Buffer and fallibly attempts to interpret it as a `Hashmap<String, serde_json::Value>`.
///
/// The JSON is fallibly checked to ensure UTF-8 formatting of any string properties.
///
/// ## Notes
///
/// This function does a memcopy from the provided Cobhan Buffer into Rust owned data.
///
/// ## Safety
///
/// Behavior is undefined if any of the following conditions are violated:
/// - The Cobhan Buffer Header size is not correctly reserved or formatted.
/// - Any of the Safety conditions of [`std::slice::from_raw_parts`][] is violated.
pub unsafe fn cbuffer_to_hashmap_json(
    buffer: *const c_char,
) -> Result<HashMap<String, Value>, i32> {
    if buffer.is_null() {
        debug_print!("cbuffer_to_hashmap_json: buffer is NULL");
        return Err(ERR_NULL_PTR);
    }
    let length = *(buffer as *const i32);
    let _reserved = buffer.offset(SIZEOF_INT32) as *const i32;
    let payload = buffer.offset(BUFFER_HEADER_SIZE) as *const u8;
    debug_print!("cbuffer_to_hashmap_json: raw length field is {}", length);

    let json_bytes = if length >= 0 {
        Cow::Borrowed(from_raw_parts(payload, length as usize))
    } else {
        debug_print!("cbuffer_to_hashmap_json: calling temp_to_vector");
        Cow::Owned(temp_to_vector(payload, length)?)
    };

    serde_json::from_slice(&json_bytes).map_err(|_e| {
        debug_print!(
            "cbuffer_to_hashmap_json: serde_json::from_slice / JSON decode failed {}",
            _e
        );
        ERR_JSON_DECODE_FAILED
    })
}

/// Takes a `Hashmap<String, serde_json::Value>` and fallibly encodes it in JSON into a provided external Cobhan Buffer.
///
/// The JSON is fallibly checked to ensure UTF-8 formatting of any string properties.
///
/// Will cause an error code if the provided Cobhan Buffer is too small.
///
/// ## Notes
///
/// This function does a memcopy from the Rust data into the provided Cobhan Buffer.
///
/// ## Safety
///
/// Behavior is undefined if any of the following conditions are violated:
/// - The Cobhan Buffer Header size is not correctly reserved or formatted.
/// - Any of the Safety conditions of [`std::slice::from_raw_parts`][] is violated.
pub unsafe fn hashmap_json_to_cbuffer(json: &HashMap<String, Value>, buffer: *mut c_char) -> i32 {
    match serde_json::to_vec(&json) {
        Ok(json_bytes) => bytes_to_cbuffer(&json_bytes, buffer),
        Err(_) => ERR_JSON_ENCODE_FAILED,
    }
}

/// Takes a `String` and fallibly encodes it into a provided external Cobhan Buffer.
///
/// Will cause an error code if the provided Cobhan Buffer is too small.
///
/// ## Notes
///
/// This function does a memcopy from the Rust data into the provided Cobhan Buffer.
///
/// ## Safety
///
/// Behavior is undefined if any of the following conditions are violated:
/// - The Cobhan Buffer Header size is not correctly reserved or formatted.
/// - Any of the Safety conditions of [`std::slice::from_raw_parts`][] is violated.
pub unsafe fn string_to_cbuffer(string: &str, buffer: *mut c_char) -> i32 {
    bytes_to_cbuffer(string.as_bytes(), buffer)
}

/// Takes a `Vec<u8>` and fallibly encodes it into a provided external Cobhan Buffer.
///
/// Will cause an error code if the provided Cobhan Buffer is too small.
///
/// ## Notes
///
/// This function does a memcopy from the Rust data into the provided Cobhan Buffer.
///
/// ## Safety
///
/// Behavior is undefined if any of the following conditions are violated:
/// - The Cobhan Buffer Header size is not correctly reserved or formatted.
/// - Any of the Safety conditions of [`std::slice::from_raw_parts`][] is violated.
pub unsafe fn bytes_to_cbuffer(bytes: &[u8], buffer: *mut c_char) -> i32 {
    if buffer.is_null() {
        debug_print!("bytes_to_cbuffer: buffer is NULL");
        return ERR_NULL_PTR;
    }

    let length = buffer as *mut i32;
    let _reserved = buffer.offset(SIZEOF_INT32) as *mut i32;
    let payload = (buffer.offset(BUFFER_HEADER_SIZE)) as *mut u8;

    let buffer_cap = *length;
    debug_print!("bytes_to_cbuffer: buffer capacity is {}", buffer_cap);

    if buffer_cap <= 0 {
        debug_print!("bytes_to_cbuffer: Invalid buffer capacity");
        return ERR_BUFFER_TOO_SMALL;
    }

    let bytes_len = bytes.len();
    debug_print!("bytes_to_cbuffer: bytes.len() is {}", bytes_len);

    if buffer_cap < (bytes_len as i32) {
        debug_print!("bytes_to_cbuffer: calling bytes_to_temp");
        return bytes_to_temp(bytes, buffer);
    }

    copy_nonoverlapping(bytes.as_ptr(), payload, bytes_len);

    *length = bytes_len as i32;

    ERR_NONE
}

/// Sets a tempfile data for a payload and writes bytes to it.
unsafe fn bytes_to_temp(bytes: &[u8], buffer: *mut c_char) -> i32 {
    // TODO: eventually replace this pattern with if-let once that is stable -jsenkpiel
    let tmp_file_path = match write_new_file(bytes) {
        Ok(t) => t,
        Err(r) => return r,
    };
    debug_print!(
        "bytes_to_temp: write_new_file wrote {} bytes to {}",
        bytes.len(),
        tmp_file_path
    );

    let length = buffer as *mut i32;
    let tmp_file_path_len = tmp_file_path.len() as i32;

    //NOTE: We explicitly test this so we don't recursively attempt to create temp files with string_to_cbuffer()
    if *length < tmp_file_path_len {
        //Temp file path won't fit in output buffer, we're out of luck
        debug_print!(
            "bytes_to_temp: temp file path {} is larger than buffer capacity {}",
            tmp_file_path,
            *length
        );
        let _ = fs::remove_file(tmp_file_path);
        return ERR_BUFFER_TOO_SMALL;
    }

    let result = string_to_cbuffer(&tmp_file_path, buffer);
    if result != ERR_NONE {
        debug_print!(
            "bytes_to_temp: failed to store temp path {} in buffer",
            tmp_file_path
        );
        let _ = fs::remove_file(tmp_file_path);
        return result;
    }

    *length = 0 - tmp_file_path_len;

    result
}

// Writes to a new named temporary file and returns the file name.
fn write_new_file(bytes: &[u8]) -> Result<String, i32> {
    let mut tmpfile = NamedTempFile::new().map_err(|_| ERR_WRITE_TEMP_FILE_FAILED)?;

    if tmpfile.write_all(bytes).is_err() {
        return Err(ERR_WRITE_TEMP_FILE_FAILED);
    };

    let (_, path) = tmpfile.keep().map_err(|_| ERR_WRITE_TEMP_FILE_FAILED)?;

    path.into_os_string()
        .into_string()
        .map_err(|_| ERR_WRITE_TEMP_FILE_FAILED)
}
