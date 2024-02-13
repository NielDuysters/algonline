var should = require("should");
var crypto = require("crypto");
var constants = require("constants");
var rsaKeyPair = require("../build/Release/rsa_keypair.node");

describe("rsa-keypair", function() {
  describe("when generating keys", function() {
    var keys;

    before(function(done) {
      keys = rsaKeyPair.generate();
      done();
    });

    it("should generate valid keys", function() {
      !!keys.should.be.ok;
    });

    it("should have a public key", function() {
      keys.should.have.property("publicKey");
      keys.publicKey.should.be.an.instanceOf(Buffer);
    });

    it("should have a private key", function() {
      keys.should.have.property("privateKey");
      keys.privateKey.should.be.an.instanceOf(Buffer);
    });

    if (crypto.publicEncrypt && crypto.privateDecrypt) {
      describe("when using the private key", function() {
        var plaintext = Buffer.from("test message");
        var ciphered = null,
          deciphered = null;

        before(function(done) {
          ciphered = crypto.publicEncrypt(
            {
              key: keys.publicKey,
              padding: constants.RSA_PKCS1_OAEP_PADDING
            },
            plaintext
          );

          deciphered = crypto.privateDecrypt(
            {
              key: keys.privateKey,
              padding: constants.RSA_PKCS1_OAEP_PADDING
            },
            ciphered
          );

          done();
        });

        it("should be able to encrypt", function() {
          !!ciphered.should.be.ok;
          ciphered.length.should.be.above(0);
        });

        it("should be able to decrypt", function() {
          !!deciphered.should.be.ok;
          deciphered.length.should.be.above(0);
        });

        it("should decrypt correctly", function() {
          deciphered.should.eql(plaintext);
        });
      });
    }
  });

  describe("when generating encrypted keys", function() {
    var keys,
      passPhrase = "test pass phrase";

    before(function(done) {
      keys = rsaKeyPair.generate(4096, 65537, passPhrase);
      done();
    });

    it("should generate valid keys", function() {
      !!keys.should.be.ok;
    });

    it("should have a public key", function() {
      keys.should.have.property("publicKey");
      keys.publicKey.should.be.an.instanceOf(Buffer);
    });

    it("should have a private key", function() {
      keys.should.have.property("privateKey");
      keys.privateKey.should.be.an.instanceOf(Buffer);
    });

    if (crypto.publicEncrypt && crypto.privateDecrypt) {
      describe("when using the private key", function() {
        var plaintext = Buffer.from("test message");
        var ciphered = null,
          deciphered = null;

        before(function(done) {
          ciphered = crypto.privateEncrypt(
            {
              key: keys.privateKey,
              passphrase: passPhrase,
              padding: constants.RSA_PKCS1_PADDING
            },
            plaintext
          );

          deciphered = crypto.publicDecrypt(
            {
              key: keys.publicKey,
              padding: constants.RSA_PKCS1_PADDING
            },
            ciphered
          );

          done();
        });

        it("should be able to encrypt", function() {
          !!ciphered.should.be.ok;
          ciphered.length.should.be.above(0);
        });

        it("should be able to decrypt", function() {
          !!deciphered.should.be.ok;
          deciphered.length.should.be.above(0);
        });

        it("should decrypt correctly", function() {
          deciphered.should.eql(plaintext);
        });
      });
    }
  });
});
