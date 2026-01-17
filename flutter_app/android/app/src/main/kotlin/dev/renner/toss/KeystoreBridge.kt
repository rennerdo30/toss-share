package dev.renner.toss

import android.content.Context
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import io.flutter.plugin.common.MethodChannel
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Bridge to Android Keystore for secure key storage.
 * Uses EncryptedSharedPreferences backed by Android Keystore.
 */
class KeystoreBridge(private val context: Context) {

    companion object {
        private const val CHANNEL_NAME = "toss.app/keystore"
        private const val PREFS_NAME = "toss_secure_prefs"
        private const val KEYSTORE_ALIAS = "toss_master_key"
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
    }

    private val encryptedPrefs by lazy {
        try {
            val masterKey = MasterKey.Builder(context, MasterKey.DEFAULT_MASTER_KEY_ALIAS)
                .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                .build()

            EncryptedSharedPreferences.create(
                context,
                PREFS_NAME,
                masterKey,
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
            )
        } catch (e: Exception) {
            // Fallback to regular SharedPreferences if encryption fails
            // This should be avoided in production but allows development to continue
            null
        }
    }

    /**
     * Set up the method channel for Flutter communication
     */
    fun setupMethodChannel(flutterEngine: io.flutter.embedding.engine.FlutterEngine) {
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL_NAME)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "store" -> {
                        val key = call.argument<String>("key")
                        val value = call.argument<String>("value") // Base64 encoded
                        if (key != null && value != null) {
                            try {
                                store(key, value)
                                result.success(true)
                            } catch (e: Exception) {
                                result.error("STORE_ERROR", e.message, null)
                            }
                        } else {
                            result.error("INVALID_ARGS", "Key and value required", null)
                        }
                    }
                    "retrieve" -> {
                        val key = call.argument<String>("key")
                        if (key != null) {
                            try {
                                val value = retrieve(key)
                                result.success(value)
                            } catch (e: Exception) {
                                result.error("RETRIEVE_ERROR", e.message, null)
                            }
                        } else {
                            result.error("INVALID_ARGS", "Key required", null)
                        }
                    }
                    "delete" -> {
                        val key = call.argument<String>("key")
                        if (key != null) {
                            try {
                                delete(key)
                                result.success(true)
                            } catch (e: Exception) {
                                result.error("DELETE_ERROR", e.message, null)
                            }
                        } else {
                            result.error("INVALID_ARGS", "Key required", null)
                        }
                    }
                    "isAvailable" -> {
                        result.success(isKeystoreAvailable())
                    }
                    "generateKey" -> {
                        val alias = call.argument<String>("alias")
                        if (alias != null) {
                            try {
                                generateSecretKey(alias)
                                result.success(true)
                            } catch (e: Exception) {
                                result.error("KEYGEN_ERROR", e.message, null)
                            }
                        } else {
                            result.error("INVALID_ARGS", "Alias required", null)
                        }
                    }
                    "encrypt" -> {
                        val alias = call.argument<String>("alias")
                        val data = call.argument<String>("data") // Base64 encoded
                        if (alias != null && data != null) {
                            try {
                                val encrypted = encryptWithKey(alias, Base64.decode(data, Base64.NO_WRAP))
                                result.success(Base64.encodeToString(encrypted, Base64.NO_WRAP))
                            } catch (e: Exception) {
                                result.error("ENCRYPT_ERROR", e.message, null)
                            }
                        } else {
                            result.error("INVALID_ARGS", "Alias and data required", null)
                        }
                    }
                    "decrypt" -> {
                        val alias = call.argument<String>("alias")
                        val data = call.argument<String>("data") // Base64 encoded
                        if (alias != null && data != null) {
                            try {
                                val decrypted = decryptWithKey(alias, Base64.decode(data, Base64.NO_WRAP))
                                result.success(Base64.encodeToString(decrypted, Base64.NO_WRAP))
                            } catch (e: Exception) {
                                result.error("DECRYPT_ERROR", e.message, null)
                            }
                        } else {
                            result.error("INVALID_ARGS", "Alias and data required", null)
                        }
                    }
                    else -> result.notImplemented()
                }
            }
    }

    /**
     * Store a value in encrypted shared preferences
     */
    fun store(key: String, value: String) {
        encryptedPrefs?.edit()?.putString(key, value)?.apply()
            ?: throw IllegalStateException("Encrypted preferences not available")
    }

    /**
     * Retrieve a value from encrypted shared preferences
     */
    fun retrieve(key: String): String? {
        return encryptedPrefs?.getString(key, null)
    }

    /**
     * Delete a value from encrypted shared preferences
     */
    fun delete(key: String) {
        encryptedPrefs?.edit()?.remove(key)?.apply()
    }

    /**
     * Check if Android Keystore is available
     */
    fun isKeystoreAvailable(): Boolean {
        return try {
            val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE)
            keyStore.load(null)
            true
        } catch (e: Exception) {
            false
        }
    }

    /**
     * Generate a secret key in Android Keystore
     */
    fun generateSecretKey(alias: String) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            val keyGenerator = KeyGenerator.getInstance(
                KeyProperties.KEY_ALGORITHM_AES,
                ANDROID_KEYSTORE
            )

            val spec = KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build()

            keyGenerator.init(spec)
            keyGenerator.generateKey()
        } else {
            throw UnsupportedOperationException("Android Keystore requires API level 23+")
        }
    }

    /**
     * Encrypt data using a key from Android Keystore
     */
    fun encryptWithKey(alias: String, data: ByteArray): ByteArray {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE)
        keyStore.load(null)

        val secretKey = keyStore.getKey(alias, null) as? SecretKey
            ?: throw IllegalArgumentException("Key not found: $alias")

        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, secretKey)

        val iv = cipher.iv
        val encrypted = cipher.doFinal(data)

        // Prepend IV to ciphertext
        return iv + encrypted
    }

    /**
     * Decrypt data using a key from Android Keystore
     */
    fun decryptWithKey(alias: String, data: ByteArray): ByteArray {
        if (data.size < 12) {
            throw IllegalArgumentException("Data too short for GCM decryption")
        }

        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE)
        keyStore.load(null)

        val secretKey = keyStore.getKey(alias, null) as? SecretKey
            ?: throw IllegalArgumentException("Key not found: $alias")

        val iv = data.sliceArray(0 until 12)
        val ciphertext = data.sliceArray(12 until data.size)

        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        val spec = GCMParameterSpec(128, iv)
        cipher.init(Cipher.DECRYPT_MODE, secretKey, spec)

        return cipher.doFinal(ciphertext)
    }
}
