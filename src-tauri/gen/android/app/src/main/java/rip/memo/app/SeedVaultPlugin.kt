package rip.memo.app

import android.app.Activity
import android.content.Intent
import android.database.Cursor
import android.net.Uri
import android.util.Base64
import android.util.Log
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/**
 * Tauri plugin for Solana Mobile Seed Vault integration.
 *
 * Provides key custody and transaction signing through the
 * Seed Vault secure execution environment on Solana Mobile devices.
 */
@TauriPlugin
class SeedVaultPlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        private const val TAG = "SeedVaultPlugin"

        // Request codes for activity results
        const val REQUEST_AUTHORIZE_SEED = 1001
        const val REQUEST_SIGN_TRANSACTION = 1002
        const val REQUEST_GET_PUBLIC_KEY = 1003

        // Singleton instance for accessing from MainActivity
        private var instance: SeedVaultPlugin? = null

        fun getInstance(): SeedVaultPlugin? = instance
        // Seed Vault Wallet API contract constants
        private const val AUTHORITY = "com.solanamobile.seedvault.wallet.v1.walletprovider"
        private const val ACTION_AUTHORIZE_SEED_ACCESS =
            "com.solanamobile.seedvault.wallet.v1.ACTION_AUTHORIZE_SEED_ACCESS"
        private const val ACTION_SIGN_TRANSACTION =
            "com.solanamobile.seedvault.wallet.v1.ACTION_SIGN_TRANSACTION"
        private const val ACTION_GET_PUBLIC_KEY =
            "com.solanamobile.seedvault.wallet.v1.ACTION_GET_PUBLIC_KEY"

        // Intent extras
        private const val EXTRA_PURPOSE = "com.solanamobile.seedvault.wallet.v1.EXTRA_PURPOSE"
        private const val EXTRA_DERIVATION_PATH = "com.solanamobile.seedvault.wallet.v1.EXTRA_DERIVATION_PATH"
        private const val EXTRA_TRANSACTION = "com.solanamobile.seedvault.wallet.v1.EXTRA_TRANSACTION"
        private const val EXTRA_AUTH_TOKEN = "com.solanamobile.seedvault.wallet.v1.EXTRA_AUTH_TOKEN"
        private const val EXTRA_PUBLIC_KEY = "com.solanamobile.seedvault.wallet.v1.EXTRA_PUBLIC_KEY"
        private const val EXTRA_SIGNATURES = "com.solanamobile.seedvault.wallet.v1.EXTRA_SIGNATURES"

        // Purpose: Solana transaction signing
        private const val PURPOSE_SIGN_SOLANA_TRANSACTION = 0

        // Content provider table URIs
        private val URI_AUTHORIZED_SEEDS = Uri.parse("content://$AUTHORITY/authorizedseeds")
        private val URI_UNAUTHORIZED_SEEDS = Uri.parse("content://$AUTHORITY/unauthorizedseeds")
        private val URI_ACCOUNTS = Uri.parse("content://$AUTHORITY/accounts")

        // Default Solana derivation path (BIP44: m/44'/501'/0'/0')
        private val DEFAULT_DERIVATION_PATH = Uri.parse("bip44:44'/501'/0'/0'")
    }

    init {
        instance = this
        Log.d(TAG, "SeedVaultPlugin initialized")
    }

    // Pending invoke references for async activity results
    private var pendingAuthorizeInvoke: Invoke? = null
    private var pendingSignInvoke: Invoke? = null
    private var pendingPublicKeyInvoke: Invoke? = null

    /**
     * Check if Seed Vault is available on this device.
     * Queries the content provider to see if it responds.
     */
    @Command
    fun isAvailable(invoke: Invoke) {
        Log.d(TAG, "isAvailable called")
        val ret = JSObject()
        try {
            val resolver = activity.contentResolver
            val cursor: Cursor? = resolver.query(
                URI_AUTHORIZED_SEEDS,
                null, null, null, null
            )
            // If we can query without exception, Seed Vault is available
            val available = cursor != null
            cursor?.close()
            ret.put("available", available)
        } catch (e: Exception) {
            // Content provider not found = Seed Vault not installed
            ret.put("available", false)
        }
        invoke.resolve(ret)
    }

    /**
     * Request authorization for a seed.
     * Launches the Seed Vault authorization UI.
     * Returns the auth token on success.
     */
    @Command
    fun authorizeSeed(invoke: Invoke) {
        Log.d(TAG, "authorizeSeed called")
        try {
            pendingAuthorizeInvoke = invoke
            val intent = Intent(ACTION_AUTHORIZE_SEED_ACCESS).apply {
                putExtra(EXTRA_PURPOSE, PURPOSE_SIGN_SOLANA_TRANSACTION)
            }
            Log.d(TAG, "Starting Seed Vault authorization activity...")
            activity.startActivityForResult(intent, REQUEST_AUTHORIZE_SEED)
            Log.d(TAG, "Activity started successfully")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to launch Seed Vault authorization", e)
            pendingAuthorizeInvoke = null
            invoke.reject("Failed to launch Seed Vault authorization: ${e.message}")
        }
    }

    /**
     * Get authorized seeds from the content provider.
     * Returns a list of authorized seed auth tokens and names.
     */
    @Command
    fun getAuthorizedSeeds(invoke: Invoke) {
        Log.d(TAG, "getAuthorizedSeeds called")
        val ret = JSObject()
        try {
            val resolver = activity.contentResolver
            val cursor: Cursor? = resolver.query(
                URI_AUTHORIZED_SEEDS,
                null, null, null, null
            )

            val seeds = mutableListOf<JSObject>()
            cursor?.use { c ->
                while (c.moveToNext()) {
                    val seed = JSObject()
                    val authTokenIdx = c.getColumnIndex("_id")
                    val nameIdx = c.getColumnIndex("name")

                    if (authTokenIdx >= 0) seed.put("authToken", c.getLong(authTokenIdx))
                    if (nameIdx >= 0) seed.put("name", c.getString(nameIdx) ?: "")

                    seeds.add(seed)
                }
            }

            ret.put("seeds", seeds.map { it.toString() }.joinToString(",", "[", "]"))
            ret.put("count", seeds.size)

            if (seeds.isNotEmpty()) {
                // Return first seed's auth token for convenience
                ret.put("authToken", seeds[0].getLong("authToken"))
            }
        } catch (e: Exception) {
            ret.put("seeds", "[]")
            ret.put("count", 0)
            ret.put("error", e.message ?: "Unknown error")
        }
        invoke.resolve(ret)
    }

    /**
     * Get accounts for an authorized seed.
     * Returns the public key for the default derivation path.
     */
    @Command
    fun getAccounts(invoke: Invoke) {
        val args = invoke.parseArgs(JSObject::class.java)
        val authToken = args.getLong("authToken")

        val ret = JSObject()
        try {
            val resolver = activity.contentResolver
            val accountsUri = Uri.withAppendedPath(URI_ACCOUNTS, authToken.toString())
            val cursor: Cursor? = resolver.query(
                accountsUri,
                null, null, null, null
            )

            val accounts = mutableListOf<JSObject>()
            cursor?.use { c ->
                while (c.moveToNext()) {
                    val account = JSObject()
                    val idIdx = c.getColumnIndex("_id")
                    val nameIdx = c.getColumnIndex("name")
                    val derivationPathIdx = c.getColumnIndex("derivation_path")
                    val publicKeyIdx = c.getColumnIndex("public_key_encoded")
                    val isUserWalletIdx = c.getColumnIndex("is_user_wallet")

                    if (idIdx >= 0) account.put("id", c.getLong(idIdx))
                    if (nameIdx >= 0) account.put("name", c.getString(nameIdx) ?: "")
                    if (derivationPathIdx >= 0) account.put("derivationPath", c.getString(derivationPathIdx) ?: "")
                    if (publicKeyIdx >= 0) account.put("publicKeyEncoded", c.getString(publicKeyIdx) ?: "")
                    if (isUserWalletIdx >= 0) account.put("isUserWallet", c.getInt(isUserWalletIdx) != 0)

                    accounts.add(account)
                }
            }

            ret.put("accounts", accounts.map { it.toString() }.joinToString(",", "[", "]"))
            ret.put("count", accounts.size)

            // Find the first account with a public key for convenience
            val firstWithKey = accounts.firstOrNull {
                it.has("publicKeyEncoded") && it.getString("publicKeyEncoded").isNotEmpty()
            }
            if (firstWithKey != null) {
                ret.put("publicKey", firstWithKey.getString("publicKeyEncoded"))
            }
        } catch (e: Exception) {
            ret.put("accounts", "[]")
            ret.put("count", 0)
            ret.put("error", e.message ?: "Unknown error")
        }
        invoke.resolve(ret)
    }

    /**
     * Sign a transaction using Seed Vault.
     * Launches the Seed Vault signing UI for user approval.
     *
     * @param authToken - The auth token for the authorized seed
     * @param transaction - Base64 encoded transaction bytes
     * @param derivationPath - Optional derivation path (defaults to m/44'/501'/0'/0')
     */
    @Command
    fun signTransaction(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(JSObject::class.java)
            val authToken = args.getLong("authToken")
            val transactionBase64 = args.getString("transaction")
            if (transactionBase64.isEmpty()) {
                return invoke.reject("Missing transaction parameter")
            }
            val derivationPath = args.optString("derivationPath", "")

            val transactionBytes = Base64.decode(transactionBase64, Base64.NO_WRAP)

            pendingSignInvoke = invoke
            val intent = Intent(ACTION_SIGN_TRANSACTION).apply {
                putExtra(EXTRA_AUTH_TOKEN, authToken)
                putExtra(EXTRA_DERIVATION_PATH,
                    if (derivationPath.isNotEmpty()) Uri.parse(derivationPath) else DEFAULT_DERIVATION_PATH
                )
                putExtra(EXTRA_TRANSACTION, arrayOf(transactionBytes))
            }
            activity.startActivityForResult(intent, REQUEST_SIGN_TRANSACTION)
        } catch (e: Exception) {
            pendingSignInvoke = null
            invoke.reject("Failed to launch Seed Vault signing: ${e.message}")
        }
    }

    /**
     * Handle activity results forwarded from MainActivity.
     * Called by MainActivity.onActivityResult().
     */
    fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        Log.d(TAG, "onActivityResult called: requestCode=$requestCode, resultCode=$resultCode")
        when (requestCode) {
            REQUEST_AUTHORIZE_SEED -> handleAuthorizeResult(resultCode, data)
            REQUEST_SIGN_TRANSACTION -> handleSignResult(resultCode, data)
            REQUEST_GET_PUBLIC_KEY -> handlePublicKeyResult(resultCode, data)
        }
    }

    private fun handleAuthorizeResult(resultCode: Int, data: Intent?) {
        val invoke = pendingAuthorizeInvoke ?: return
        pendingAuthorizeInvoke = null

        if (resultCode != Activity.RESULT_OK) {
            invoke.reject("Seed authorization was cancelled or failed")
            return
        }

        val ret = JSObject()
        try {
            val authToken = data?.getLongExtra(EXTRA_AUTH_TOKEN, -1) ?: -1
            if (authToken == -1L) {
                invoke.reject("No auth token returned from Seed Vault")
                return
            }
            ret.put("authToken", authToken)
            invoke.resolve(ret)
        } catch (e: Exception) {
            invoke.reject("Failed to process authorization result: ${e.message}")
        }
    }

    private fun handleSignResult(resultCode: Int, data: Intent?) {
        val invoke = pendingSignInvoke ?: return
        pendingSignInvoke = null

        if (resultCode != Activity.RESULT_OK) {
            invoke.reject("Transaction signing was cancelled or failed")
            return
        }

        val ret = JSObject()
        try {
            @Suppress("DEPRECATION")
            val signatures = data?.getSerializableExtra(EXTRA_SIGNATURES)

            if (signatures is Array<*>) {
                val sigArray = signatures.filterIsInstance<ByteArray>()
                if (sigArray.isNotEmpty()) {
                    // Return the first signature as base64
                    val signatureBase64 = Base64.encodeToString(sigArray[0], Base64.NO_WRAP)
                    ret.put("signature", signatureBase64)
                    ret.put("signatureCount", sigArray.size)
                    invoke.resolve(ret)
                    return
                }
            }

            invoke.reject("No signatures returned from Seed Vault")
        } catch (e: Exception) {
            invoke.reject("Failed to process signing result: ${e.message}")
        }
    }

    private fun handlePublicKeyResult(resultCode: Int, data: Intent?) {
        val invoke = pendingPublicKeyInvoke ?: return
        pendingPublicKeyInvoke = null

        if (resultCode != Activity.RESULT_OK) {
            invoke.reject("Public key request was cancelled or failed")
            return
        }

        val ret = JSObject()
        try {
            val publicKey = data?.getByteArrayExtra(EXTRA_PUBLIC_KEY)
            if (publicKey != null) {
                ret.put("publicKey", Base64.encodeToString(publicKey, Base64.NO_WRAP))
                invoke.resolve(ret)
            } else {
                invoke.reject("No public key returned from Seed Vault")
            }
        } catch (e: Exception) {
            invoke.reject("Failed to process public key result: ${e.message}")
        }
    }
}
