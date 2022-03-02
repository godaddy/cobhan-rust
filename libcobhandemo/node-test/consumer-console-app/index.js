import libcobhandemo from 'libcobhandemo';

libcobhandemo.spawnThread()

console.log('Start Counter: ' + libcobhandemo.readCounter())

console.log(libcobhandemo.toUpper('Initial value'));

let output = libcobhandemo.filterJsonObject({ test: 'foo', test2: 'kittens' } , 'foo');
console.log(output);

// Intentionally showing integer behavior Number is truncation, not rounding
console.log(libcobhandemo.addInt32(2.9, 2.0));
console.log(libcobhandemo.addInt64(2.9, 2.0));

// Double is the same as Number
console.log(libcobhandemo.addDouble(2.9, 2.0));


// Test using a Promise to call a blocking function
console.log('Start sleeping');
libcobhandemo.sleepTest(2).then(() => {
        console.log('Finished sleeping');
        console.log('Final Counter: ' + libcobhandemo.readCounter());
    })
