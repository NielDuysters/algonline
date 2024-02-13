#include <node_api.h>

#include <openssl/ssl.h>
#include <openssl/err.h>
#include <openssl/pem.h>

#define PASSPHRASE_LEN 100

static RSA *generateKey(int num, unsigned long e)
{
#if OPENSSL_VERSION_NUMBER < 0x009080001
	return RSA_generate_key(num, e, NULL, NULL);
#else
	BIGNUM *eBig = BN_new();

	if (eBig == NULL)
	{
		return NULL;
	}

	if (!BN_set_word(eBig, e))
	{
		BN_free(eBig);
		return NULL;
	}

	RSA *result = RSA_new();

	if (result == NULL)
	{
		BN_free(eBig);
		return NULL;
	}

	if (RSA_generate_key_ex(result, num, eBig, NULL) < 0)
	{
		RSA_free(result);
		result = NULL;
	}

	BN_free(eBig);

	return result;
#endif
}

napi_value Generate(napi_env env, napi_callback_info info)
{
	size_t argc = 3, passPhraseLength = 0;
	napi_value argv[3], publicKey, privateKey, result;
	napi_status status;
	int modulusBits = 2048, exponent = 65537, number = 0, success = 0;
	char passPhrase[PASSPHRASE_LEN], *data = NULL;
	long length = 0;
	void *out;
	const EVP_CIPHER *cipher = NULL;

	status = napi_get_cb_info(env, info, &argc, argv, NULL, NULL);
	if (napi_ok != status)
	{
		napi_throw_error(env, NULL, "Failed to parse arguments");
	}

	// module bits param
	status = napi_get_value_int32(env, argv[0], &number);
	if (napi_ok == status)
	{
		modulusBits = number;
		if (modulusBits < 512)
		{
			napi_throw_type_error(env, NULL, "Expected modulus bit count bigger than 512.");
		}
	}

	// exponent param
	status = napi_get_value_int32(env, argv[1], &number);
	if (napi_ok == status)
	{
		exponent = number;
		if (exponent < 0)
		{
			napi_throw_type_error(env, NULL, "Expected positive exponent.");
		}
		if ((exponent & 1) == 0)
		{
			napi_throw_type_error(env, NULL, "Expected odd exponent.");
		}
	}

	// pass phrase param
	status = napi_get_value_string_utf8(env, argv[2], passPhrase, PASSPHRASE_LEN, &passPhraseLength);
	if (napi_ok == status)
	{
		cipher = (EVP_CIPHER *)EVP_des_ede3_cbc();
	}

	RSA *rsa = generateKey(modulusBits, (unsigned int)exponent);

	if (!rsa)
	{
		napi_throw_error(env, NULL, "Failed creating RSA context.");
	}

	BIO *publicBio = BIO_new(BIO_s_mem());
	BIO *privateBio = BIO_new(BIO_s_mem());

	if (!publicBio || !privateBio)
	{
		if (publicBio)
		{
			BIO_vfree(publicBio);
		}
		if (privateBio)
		{
			BIO_vfree(privateBio);
		}
		RSA_free(rsa);
		napi_throw_error(env, NULL, "Failed to allocate OpenSSL buffers.");
	}

	if (!PEM_write_bio_RSA_PUBKEY(publicBio, rsa))
	{
		BIO_vfree(publicBio);
		BIO_vfree(privateBio);
		RSA_free(rsa);
		napi_throw_error(env, NULL, "Failed exporting public key.");
	}

	if (passPhraseLength > 0)
	{
		success = PEM_write_bio_RSAPrivateKey(privateBio, rsa, cipher, (unsigned char *)passPhrase, (int)passPhraseLength, NULL, NULL);
	}
	else
	{
		success = PEM_write_bio_RSAPrivateKey(privateBio, rsa, cipher, NULL, 0, NULL, NULL);
	}
	if (!success)
	{
		BIO_vfree(publicBio);
		BIO_vfree(privateBio);
		RSA_free(rsa);
		napi_throw_error(env, NULL, "Failed exporting private key.");
	}

	length = BIO_get_mem_data(publicBio, &data);
	status = napi_create_buffer_copy(env, length, (void *)data, &out, &publicKey);
	if (napi_ok != status)
	{
		BIO_vfree(publicBio);
		BIO_vfree(privateBio);
		RSA_free(rsa);
		napi_throw_error(env, NULL, "Failed writing public key.");
	}

	length = BIO_get_mem_data(privateBio, &data);
	status = napi_create_buffer_copy(env, length, (void *)data, &out, &privateKey);
	if (napi_ok != status)
	{
		BIO_vfree(publicBio);
		BIO_vfree(privateBio);
		RSA_free(rsa);
		napi_throw_error(env, NULL, "Failed writing private key.");
	}

	BIO_vfree(publicBio);
	BIO_vfree(privateBio);
	RSA_free(rsa);

	status = napi_create_object(env, &result);
	if (napi_ok != status)
	{
		napi_throw_error(env, NULL, "Failed creating result.");
	}

	status = napi_set_named_property(env, result, "publicKey", publicKey);
	if (napi_ok != status)
	{
		napi_throw_error(env, NULL, "Failed adding publicKey to result.");
	}

	status = napi_set_named_property(env, result, "privateKey", privateKey);
	if (napi_ok != status)
	{
		napi_throw_error(env, NULL, "Failed adding privateKey to result.");
	}

	return result;
}

napi_value Init(napi_env env, napi_value exports)
{
	napi_status status;
	napi_property_descriptor desc =
			{"generate", NULL, Generate, NULL, NULL, NULL, napi_default, NULL};
	status = napi_define_properties(env, exports, 1, &desc);
	if (status != napi_ok)
		return NULL;
	return exports;
}

NAPI_MODULE(NODE_GYP_MODULE_NAME, Init);
