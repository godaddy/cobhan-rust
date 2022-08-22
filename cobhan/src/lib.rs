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

const ERR_NONE: i32 = 0;
pub const ERR_NONE: i32 = 0;

//One of the provided pointers is NULL / nil / 0
const ERR_NULL_PTR: i32 = -1;
pub const ERR_NULL_PTR: i32 = -1;

//One of the provided buffer lengths is too large
const ERR_BUFFER_TOO_LARGE: i32 = -2;
pub const ERR_BUFFER_TOO_LARGE: i32 = -2;

//One of the provided buffers was too small
const ERR_BUFFER_TOO_SMALL: i32 = -3;
pub const ERR_BUFFER_TOO_SMALL: i32 = -3;

//Failed to copy a buffer (copy length != expected length)
//const ERR_COPY_FAILED: i32 = -4;
pub const ERR_COPY_FAILED: i32 = -4;

//Failed to decode a JSON buffer
const ERR_JSON_DECODE_FAILED: i32 = -5;
pub const ERR_JSON_DECODE_FAILED: i32 = -5;

//Failed to encode to JSON buffer
const ERR_JSON_ENCODE_FAILED: i32 = -6;
pub const ERR_JSON_ENCODE_FAILED: i32 = -6;

const ERR_INVALID_UTF8: i32 = -7;
pub const ERR_INVALID_UTF8: i32 = -7;

const ERR_READ_TEMP_FILE_FAILED: i32 = -8;
pub const ERR_READ_TEMP_FILE_FAILED: i32 = -8;

const ERR_WRITE_TEMP_FILE_FAILED: i32 = -9;
pub const ERR_WRITE_TEMP_FILE_FAILED: i32 = -9;

const BUFFER_HEADER_SIZE: isize = 64 / 8; // 64 bit buffer header provides 8 byte alignment for data pointers
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

/// ## Safety
/// tbd
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

/// ## Safety
/// tbd
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

/// ## Safety
/// tbd
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

/// ## Safety
/// tbd
pub unsafe fn hashmap_json_to_cbuffer(json: &HashMap<String, Value>, buffer: *mut c_char) -> i32 {
    match serde_json::to_vec(&json) {
        Ok(json_bytes) => bytes_to_cbuffer(&json_bytes, buffer),
        Err(_) => ERR_JSON_ENCODE_FAILED,
    }
}

/// ## Safety
/// tbd
pub unsafe fn string_to_cbuffer(string: &str, buffer: *mut c_char) -> i32 {
    bytes_to_cbuffer(string.as_bytes(), buffer)
}

/// ## Safety
/// tbd
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

unsafe fn write_new_file(bytes: &[u8]) -> Result<String, i32> {
    let mut tmpfile = NamedTempFile::new().map_err(|_| ERR_WRITE_TEMP_FILE_FAILED)?;

    if tmpfile.write_all(bytes).is_err() {
        return Err(ERR_WRITE_TEMP_FILE_FAILED);
    };

    let (_, path) = tmpfile.keep().map_err(|_| ERR_WRITE_TEMP_FILE_FAILED)?;

    path.into_os_string()
        .into_string()
        .map_err(|_| ERR_WRITE_TEMP_FILE_FAILED)
}
