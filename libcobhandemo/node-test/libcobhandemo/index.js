import cobhan from 'cobhan'

const libcobhandemo = cobhan.load_platform_library('node_modules/libcobhandemo/binaries', 'libcobhandemo', {
    'sleepTest': ['void', ['int32']],
    'addInt32': ['int32', ['int32', 'int32']],
    'addInt64': ['int64', ['int64', 'int64']],
    'addDouble': ['double', ['double', 'double']],
    'toUpper': ['int32', ['pointer', 'pointer']],
    'filterJson': ['int32', ['pointer', 'pointer', 'pointer']],
    'spawnThread': ['void', []],
    'readCounter': ['int32', []]
    });

function spawnThread() {
    libcobhandemo.spawnThread();
}

/**
* @return {number}
*/
function readCounter() {
    return libcobhandemo.readCounter();
}

/**
* @param {object} input
* @param {string} disallowedValue
* @return {object}
*/
function filterJsonObject(input, disallowedValue) {
    const json = JSON.stringify(input);
    const inputBuffer = cobhan.string_to_cbuffer(json);
    const disallowedBuffer = cobhan.string_to_cbuffer(disallowedValue);
    const outputBuffer = cobhan.allocate_cbuffer(json.length);

    const result = libcobhandemo.filterJson(inputBuffer, disallowedBuffer, outputBuffer);
    if (result < 0) {
        throw new Error('filterJson failed: ' + result);
    }

    return JSON.parse(cobhan.cbuffer_to_string(outputBuffer));
}

/**
* @param {string} inputJson
* @param {string} disallowedValue
* @return {string}
*/
function filterJsonString(inputJson, disallowedValue) {
    const inputBuffer = cobhan.string_to_cbuffer(inputJson);
    const disallowedBuffer = cobhan.string_to_cbuffer(disallowedValue);
    const outputBuffer = cobhan.allocate_cbuffer(inputJson.length);

    const result = libcobhandemo.filterJson(inputBuffer, disallowedBuffer, outputBuffer);
    if (result < 0) {
        throw new Error('filterJson failed: ' + result);
    }

    return cobhan.cbuffer_to_string(outputBuffer);
}

/**
* @param {string} input
* @return {string}
*/
function toUpper(input) {
    const inputBuffer = cobhan.string_to_cbuffer(input);
    const outputBuffer = cobhan.allocate_cbuffer(input.length);

  const result = libcobhandemo.toUpper(inputBuffer, outputBuffer);
  if (result < 0) {
    throw new Error('toUpper failed: ' + result);
  }

  return cobhan.cbuffer_to_string(outputBuffer);
}

/**
* @param {number} x
* @param {number} y
* @return {number}
*/
function addInt32(x, y) {
  return libcobhandemo.addInt32(x, y);
}

/**
* @param {number} x
* @param {number} y
* @return {number}
*/
function addInt64(x, y) {
  return libcobhandemo.addInt64(x, y);
}

/**
* @param {number} x
* @param {number} y
* @return {number}
*/
function addDouble(x, y) {
  return libcobhandemo.addDouble(x, y);
}

/**
* @param {number} seconds
* @return {Promise}
*/
function sleepTest(seconds) {
  return new Promise((resolve) => {
    libcobhandemo.sleepTest.async(seconds, () => {
    resolve();
    });
  });
}

export default { spawnThread, readCounter, filterJsonObject, filterJsonString, toUpper, sleepTest, addInt32, addInt64, addDouble };
