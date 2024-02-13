# node-rsa-keypair

[![NPM version][npm-image]][npm-url] [![MIT License][license-image]][license-url] [![Build Status][travis-image]][travis-url] [![Dependency Status][daviddm-image]][daviddm-url]

Generates a RSA keypair using native OpenSSL library.

This is a fork of [rsa-keygen](https://github.com/sunjith/node-rsa-keygen) with support for encrypting the generated private key with a given pass phrase. The dependencies have also been updated as per the pull request [Update deps](https://github.com/sunjith/node-rsa-keygen/pull/6) by [omsmith](https://github.com/omsmith) and [calvinmetcalf](https://github.com/calvinmetcalf) which was not merged in to the original rsa-keygen.

This code is loosely based on [ursa](https://github.com/Medium/ursa) RSA generation code.

Thanks to all the developers who have contributed to the above projects.

## N-API Example

The module and code also serves as a simple example to create native addon module in C/C++ using [N-API](https://nodejs.org/api/n-api.html).

## Upgrade Notes

Version 2.x uses [N-API](https://nodejs.org/api/n-api.html). For older node versions that do not support [N-API](https://nodejs.org/api/n-api.html), use version 1.0.1:

```sh
npm install --save rsa-keypair@1.0.1
```

## History

The node `crypto` library has encrypt and decrypt functions, we don't need to rely on any external libraries for public-key cryptography.

## Usage

Install the library using npm:

```sh
npm install --save rsa-keypair
```

Or install using yarn:

```sh
yarn add rsa-keypair
```

Use in your code:

```javascript
var rsaKeyPair = require("rsa-keypair");
var keys = rsaKeyPair.generate();
```

ES6:

```javascript
import rsaKeyPair from "rsa-keypair";
const keys = rsaKeyPair.generate();
```

## API

```javascript
/**
 * Generate RSA key pair.
 * @param {Number} modulusBits - The RSA key size. Minimum: 512, default: 2048.
 * @param {Number} exponent - The public exponent. Should be relatively prime to p-1 for all primes p which divide the modulus. Default: 65537.
 * @param {String} passPhrase - The pass phrase to encrypt the RSA private key. If not specified the private key shall be unencrypted. Even passing an empty string will cause the private key to be encrypted. Default: no pass phrase.
 * @return {Object} The object containing the private key in property 'privateKey' and the public key in property 'publicKey'. Note: if 'passPhrase' was passed to encrypt the private key, it is not retuned in the result object.
 */
function generate(modulusBits, exponent, passPhrase);
```

## Examples

```javascript
var crypto = require("crypto");
var rsaKeyPair = require("rsa-keypair");

var keys = rsaKeyPair.generate();

var result = crypto.publicEncrypt(
 {
  key: keys.publicKey
 },
 new Buffer("Hello world!")
);
// <Crypted Buffer>

var plaintext = crypto.privateDecrypt(
 {
  key: keys.privateKey
 },
 result
);
// Hello world!
```

```javascript
var crypto = require("crypto");
var rsaKeyPair = require("rsa-keypair");

var keys = rsaKeyPair.generate(4096, 65537, "top secret");
// Generates a 4096-bit RSA key pair with "top secret" as the pass phrase to encrypt the private key

var result = crypto.privateEncrypt(
 {
  key: keys.privateKey,
  passphrase: "top secret",
  padding: crypto.constants.RSA_PKCS1_PADDING
 },
 new Buffer("Hello world!")
);
// <Crypted Buffer>

var plaintext = crypto.publicDecrypt(
 {
  key: keys.publicKey,
  padding: crypto.constants.RSA_PKCS1_PADDING
 },
 result
);
// Hello world!
```

```javascript
var rsaKeyPair = require("rsa-keypair");

var keys = rsaKeyPair.generate(4096, 65537, "top secret");
// Generates a 4096-bit RSA key pair with "top secret" as the pass phrase to encrypt the private key

var publicKeyStr = keys.publicKey.toString();
// The public key string in PEM format which may be written to a file

var privateKeyStr = keys.privateKey.toString();
// The encrypted private key in PEM format which may be written to a file
```

## Develop

```sh
git clone https://github.com/sunjith/node-rsa-keypair
cd node-rsa-keypair

# Note: You may use yarn instead of npm in each of the following commands
# Install dependencies and build
npm install

# Build only
npm start

# Run tests
npm run test
```

[npm-image]: https://badge.fury.io/js/rsa-keypair.svg
[npm-url]: https://npmjs.org/package/rsa-keypair
[license-image]: http://img.shields.io/badge/license-MIT-blue.svg?style=flat
[license-url]: LICENSE
[travis-image]: https://travis-ci.com/sunjith/node-rsa-keypair.svg?branch=master
[travis-url]: https://travis-ci.com/sunjith/node-rsa-keypair
[daviddm-image]: https://david-dm.org/sunjith/node-rsa-keypair.svg?theme=shields.io
[daviddm-url]: https://david-dm.org/sunjith/node-rsa-keypair
