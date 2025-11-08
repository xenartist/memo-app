use serde::{Serialize, Deserialize};
use crate::core::encrypt;
use crate::core::rpc_base::RpcConnection;
use crate::core::rpc_mint::MintConfig;
use crate::core::rpc_profile::{UserProfile, parse_user_profile_new};
use crate::core::rpc_project::{ProjectInfo, ProjectStatistics, ProjectBurnLeaderboardResponse};
use crate::core::rpc_burn::{UserGlobalBurnStats};
use crate::core::network_config::{NetworkType, clear_network};
use crate::core::backpack::{BackpackWallet, BackpackError};
use web_sys::js_sys::Date;
use secrecy::{Secret, ExposeSecret};
use zeroize::{Zeroize, Zeroizing};
use hex;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use serde_json;
use std::fmt;
use std::str::FromStr;
use log;
use base64;

/// Wallet type for the session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WalletType {
    /// Internal wallet (mnemonic + password encrypted)
    Internal,
    /// Backpack web wallet
    Backpack,
}

#[derive(Debug, Clone)]
pub enum SessionError {
    Encryption(String),
    Expired,
    InvalidPassword,
    NotInitialized,
    InvalidData(String),
    ProfileError(String),
    BackpackError(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::Encryption(msg) => write!(f, "Encryption error: {}", msg),
            SessionError::Expired => write!(f, "Session expired"),
            SessionError::InvalidPassword => write!(f, "Invalid password"),
            SessionError::NotInitialized => write!(f, "Session not initialized"),
            SessionError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            SessionError::ProfileError(msg) => write!(f, "Profile error: {}", msg),
            SessionError::BackpackError(msg) => write!(f, "Backpack wallet error: {}", msg),
        }
    }
}

impl From<BackpackError> for SessionError {
    fn from(error: BackpackError) -> Self {
        SessionError::BackpackError(error.to_string())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SessionConfig {
    // session timeout in minutes, None means never expire
    timeout_minutes: Option<u32>,
    // minimum amount to confirm in lamports
    confirm_threshold: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout_minutes: None,  // never expire by default
            confirm_threshold: 1_000_000_000,  // default 1 SOL confirmation
        }
    }
}

#[derive(Clone)]
pub struct Session {
    // session config
    config: SessionConfig,
    // session start time
    start_time: f64,
    // wallet type (Internal or Backpack)
    wallet_type: WalletType,
    // encrypted seed (only for Internal wallet)
    encrypted_seed: Option<String>,
    // session key (only for Internal wallet)
    session_key: Option<Secret<String>>,
    // backpack public key (only for Backpack wallet)
    backpack_pubkey: Option<String>,
    // UI locked
    ui_locked: bool,
    // user profile
    user_profile: Option<UserProfile>,
    // cached pubkey
    cached_pubkey: Option<String>,
    // balance information
    sol_balance: f64,
    token_balance: f64,
    // balance update trigger
    balance_update_needed: bool,
    // user global burn stats
    user_burn_stats: Option<UserGlobalBurnStats>,
    // network type for this session (set during login, immutable after that)
    network: Option<NetworkType>,
}

impl Session {
    pub fn new(config: Option<SessionConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            start_time: Date::now(),
            wallet_type: WalletType::Internal, // Default to Internal
            encrypted_seed: None,
            session_key: None,
            backpack_pubkey: None,
            ui_locked: false,
            user_profile: None,
            cached_pubkey: None,
            sol_balance: 0.0,
            token_balance: 0.0,
            balance_update_needed: false,
            user_burn_stats: None,
            network: None,
        }
    }
    
    /// Get the wallet type for this session
    pub fn get_wallet_type(&self) -> &WalletType {
        &self.wallet_type
    }
    
    /// Check if session is using Backpack wallet
    pub fn is_backpack(&self) -> bool {
        self.wallet_type == WalletType::Backpack
    }
    
    /// Check if session is using internal wallet
    pub fn is_internal_wallet(&self) -> bool {
        self.wallet_type == WalletType::Internal
    }
    
    /// Set network for this session (called during login)
    pub fn set_network(&mut self, network: NetworkType) {
        self.network = Some(network);
        log::info!("Session network set to: {}", network.display_name());
    }
    
    /// Get network for this session
    pub fn get_network(&self) -> Option<NetworkType> {
        self.network
    }
    
    /// Check if session has network set
    pub fn has_network(&self) -> bool {
        self.network.is_some()
    }
    
    /// Logout and clear session
    pub fn logout(&mut self) {
        // Check if Backpack wallet BEFORE clearing wallet_type
        let is_backpack = self.is_backpack();
        
        // Clear all session data
        self.wallet_type = WalletType::Internal; // Reset to default
        self.encrypted_seed = None;
        self.session_key = None;
        self.backpack_pubkey = None;
        self.user_profile = None;
        self.cached_pubkey = None;
        self.sol_balance = 0.0;
        self.token_balance = 0.0;
        self.balance_update_needed = false;
        self.user_burn_stats = None;
        self.network = None;
        
        // If Backpack wallet, disconnect
        if is_backpack {
            wasm_bindgen_futures::spawn_local(async {
                if let Err(e) = BackpackWallet::disconnect().await {
                    log::warn!("Failed to disconnect Backpack wallet: {}", e);
                }
            });
        }
        
        // Clear global network configuration
        clear_network();
        
        log::info!("Session logged out. Network cleared.");
    }

    /// Initialize session with internal wallet (mnemonic + password)
    /// 
    /// This method decrypts the seed using user password and re-encrypts it using a session key.
    pub async fn initialize(&mut self, encrypted_seed: &str, password: &str) -> Result<(), SessionError> {
        // decrypt original seed
        let seed = encrypt::decrypt(encrypted_seed, password)
            .map_err(|e| SessionError::Encryption(e.to_string()))?;

        // generate new session key
        let session_key = encrypt::generate_random_key();

        // re-encrypt seed using session key
        let session_encrypted_seed = encrypt::encrypt(&seed, session_key.expose_secret())
            .map_err(|e| SessionError::Encryption(e.to_string()))?;

        // get pubkey
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(e.to_string()))?;
        
        let seed: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (_, pubkey) = crate::core::wallet::derive_keypair_from_seed(
            &seed,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption("Failed to derive keypair".to_string()))?;

        // save session info (Internal wallet)
        self.wallet_type = WalletType::Internal;
        self.session_key = Some(session_key);
        self.encrypted_seed = Some(session_encrypted_seed);
        self.backpack_pubkey = None;
        self.start_time = Date::now();
        self.cached_pubkey = Some(pubkey.clone());

        log::info!("Session initialized with internal wallet: {}", pubkey);
        Ok(())
    }

    /// Initialize session with Backpack wallet
    /// 
    /// This method connects to Backpack wallet and initializes the session with the connected public key.
    /// No private key or seed is stored - all signing is done through Backpack.
    pub async fn initialize_with_backpack(&mut self) -> Result<String, SessionError> {
        log::info!("Initializing session with Backpack wallet...");
        
        // Check if Backpack is installed
        if !BackpackWallet::is_installed() {
            return Err(SessionError::BackpackError(
                "Backpack wallet is not installed. Please install it from https://backpack.app".to_string()
            ));
        }

        // Connect to Backpack and get public key
        let pubkey = BackpackWallet::connect().await?;
        
        log::info!("Connected to Backpack wallet: {}", &pubkey);

        // Validate the public key
        Pubkey::from_str(&pubkey)
            .map_err(|e| SessionError::InvalidData(format!("Invalid public key from Backpack: {}", e)))?;

        // Save session info (Backpack wallet)
        self.wallet_type = WalletType::Backpack;
        self.backpack_pubkey = Some(pubkey.clone());
        self.cached_pubkey = Some(pubkey.clone());
        self.encrypted_seed = None;
        self.session_key = None;
        self.start_time = Date::now();

        log::info!("Session initialized with Backpack wallet");
        Ok(pubkey)
    }

    // check if session is expired
    pub fn is_expired(&self) -> bool {
        match self.config.timeout_minutes {
            None => false, // never expire
            Some(timeout_minutes) => {
                let current_time = Date::now();
                let elapsed_minutes = (current_time - self.start_time) / (60.0 * 1000.0);
                elapsed_minutes > timeout_minutes as f64
            }
        }
    }

    // get decrypted seed (if session is valid)
    pub fn get_seed(&self) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        match (&self.encrypted_seed, &self.session_key) {
            (Some(encrypted_seed), Some(session_key)) => {
                encrypt::decrypt(encrypted_seed, session_key.expose_secret())
                    .map_err(|e| SessionError::Encryption(e.to_string()))
            },
            _ => Err(SessionError::NotInitialized),
        }
    }

    // check if operation needs additional password confirmation
    pub fn needs_confirmation(&self, amount: u64) -> bool {
        amount >= self.config.confirm_threshold
    }

    // verify password (for operations that need confirmation)
    pub fn verify_password(&self, password: &str, original_encrypted_seed: &str) -> Result<bool, SessionError> {
        // try to decrypt original encrypted seed
        encrypt::decrypt(original_encrypted_seed, password)
            .map(|_| true)
            .map_err(|_| SessionError::InvalidPassword)
    }

    // refresh session time
    pub fn refresh(&mut self) {
        self.start_time = Date::now();
    }

    // clear session data
    pub fn clear(&mut self) {
        // clear sensitive data
        self.session_key = None; // Secret will be dropped automatically
        self.encrypted_seed = None;
        self.cached_pubkey = None;
        self.user_profile = None;
        self.sol_balance = 0.0;
        self.token_balance = 0.0;
        self.balance_update_needed = false;
        self.ui_locked = false;
        self.user_burn_stats = None;
    }

    // update config
    pub fn update_config(&mut self, config: SessionConfig) {
        self.config = config;
    }

    // get public key
    pub fn get_public_key(&self) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        match &self.cached_pubkey {
            Some(pubkey) => Ok(pubkey.clone()),
            None => Err(SessionError::NotInitialized),
        }
    }

    // lock UI
    pub fn lock_ui(&mut self) {
        self.ui_locked = true;
    }

    pub fn unlock_ui(&mut self, password: &str, original_encrypted_seed: &str) -> Result<(), SessionError> {
        match self.verify_password(password, original_encrypted_seed) {
            Ok(true) => {
                self.ui_locked = false;
                Ok(())
            },
            Ok(false) => Err(SessionError::InvalidPassword),
            Err(e) => Err(e),
        }
    }

    // check if UI is locked
    pub fn can_access_ui(&self) -> bool {
        !self.ui_locked
    }

    // get user profile
    pub fn get_user_profile(&self) -> Option<UserProfile> {
        self.user_profile.clone()
    }

    // set user profile
    pub fn set_user_profile(&mut self, profile: Option<UserProfile>) {
        self.user_profile = profile;
    }

    // initialize session with seed
    pub async fn initialize_with_seed(&mut self, seed: &str) -> Result<(), SessionError> {
        // generate new session key
        let session_key = encrypt::generate_random_key();

        // re-encrypt seed using session key
        let session_encrypted_seed = encrypt::encrypt(seed, session_key.expose_secret())
            .map_err(|e| SessionError::Encryption(e.to_string()))?;

        // get pubkey
        let seed_bytes = hex::decode(seed)
            .map_err(|e| SessionError::Encryption(e.to_string()))?;
        
        let seed: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (_, pubkey) = crate::core::wallet::derive_keypair_from_seed(
            &seed,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption("Failed to derive keypair".to_string()))?;

        // save session info
        self.session_key = Some(session_key);
        self.encrypted_seed = Some(session_encrypted_seed.to_string());
        self.start_time = Date::now();
        self.cached_pubkey = Some(pubkey);

        Ok(())
    }

    // fetch and cache user profile (updated for new profile system)
    pub async fn fetch_and_cache_user_profile(&mut self) -> Result<Option<UserProfile>, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let pubkey = self.get_public_key()?;
        let rpc = RpcConnection::new();

        match rpc.get_profile(&pubkey).await {
            Ok(Some(profile)) => {
                log::info!("Successfully fetched and cached user profile");
                self.user_profile = Some(profile.clone());
                Ok(Some(profile))
            },
            Ok(None) => {
                log::info!("User profile not found for pubkey: {}", pubkey);
                self.user_profile = None;
                Ok(None)
            },
            Err(e) => {
                log::error!("Failed to fetch user profile: {}", e);
                Err(SessionError::ProfileError(format!("RPC error: {}", e)))
            }
        }
    }

    /// Create user profile
    pub async fn create_profile(
        &mut self,
        burn_amount: u64,
        username: String,
        image: String,
        about_me: Option<String>,
    ) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let rpc = RpcConnection::new();
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        log::info!("Building create profile transaction...");
        let mut transaction = rpc.build_create_profile_transaction(&pubkey, burn_amount, &username, &image, about_me).await
            .map_err(|e| SessionError::ProfileError(format!("Failed to build transaction: {}", e)))?;
        
        log::info!("Signing transaction in Session...");
        self.sign_transaction(&mut transaction).await?;
        
        log::info!("Sending signed transaction...");
        let tx_hash = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::ProfileError(format!("Failed to send transaction: {}", e)))?;
        
        log::info!("Profile created successfully: {}", tx_hash);
        let _ = self.fetch_and_cache_user_profile().await;
        
        Ok(tx_hash)
    }

    /// Update user profile
    pub async fn update_profile(
        &mut self,
        burn_amount: u64,
        username: Option<String>,
        image: Option<String>,
        about_me: Option<String>, 
    ) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let rpc = RpcConnection::new();
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        // Convert about_me to nested Option
        let about_me_nested = match about_me {
            None => None,
            Some(text) if text.is_empty() => Some(None),
            Some(text) => Some(Some(text)),
        };
        
        log::info!("Building update profile transaction...");
        let mut transaction = rpc.build_update_profile_transaction(&pubkey, burn_amount, username, image, about_me_nested).await
            .map_err(|e| SessionError::ProfileError(format!("Failed to build transaction: {}", e)))?;
        
        log::info!("Signing transaction in Session...");
        self.sign_transaction(&mut transaction).await?;
        
        log::info!("Sending signed transaction...");
        let tx_hash = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::ProfileError(format!("Failed to send transaction: {}", e)))?;
        
        log::info!("Profile updated successfully: {}", tx_hash);
        let _ = self.fetch_and_cache_user_profile().await;
        
        Ok(tx_hash)
    }

    // delete user profile
    pub async fn delete_profile(&mut self) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let rpc = RpcConnection::new();
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        log::info!("Building delete profile transaction...");
        let mut transaction = rpc.build_delete_profile_transaction(&pubkey).await
            .map_err(|e| SessionError::ProfileError(format!("Failed to build transaction: {}", e)))?;
        
        log::info!("Signing transaction in Session...");
        self.sign_transaction(&mut transaction).await?;
        
        log::info!("Sending signed transaction...");
        let tx_hash = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::ProfileError(format!("Failed to send transaction: {}", e)))?;
        
        log::info!("Profile deleted successfully: {}", tx_hash);
        self.user_profile = None;
        
        Ok(tx_hash)
    }

    /// ⚠️ DEPRECATED - DO NOT USE ⚠️
    /// 
    /// This method is deprecated and should not be used as it exposes the private key
    /// outside of the Session context, which is a security risk.
    /// 
    /// **Security Issues:**
    /// - Returns keypair bytes containing the full private key
    /// - The returned Vec<u8> is not automatically zeroized
    /// - Caller is responsible for secure memory cleanup (often forgotten)
    /// - Private key exists in memory longer than necessary
    /// 
    /// **Migration:**
    /// All transaction signing should use the secure `sign_transaction()` pattern:
    /// 1. RPC builds unsigned transaction
    /// 2. Session signs transaction internally (using `sign_transaction`)
    /// 3. RPC sends signed transaction
    /// 
    /// This method is kept only for backward compatibility and will be removed in the future.
    #[deprecated(
        since = "0.2.0",
        note = "Use sign_transaction() instead. This method exposes private keys unsafely."
    )]
    #[allow(dead_code)]
    fn get_keypair_bytes(&self) -> Result<Vec<u8>, SessionError> {
        log::warn!("SECURITY WARNING: get_keypair_bytes() is deprecated and unsafe. Migrate to sign_transaction().");
        
        let seed = self.get_seed()?;
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(format!("Failed to decode seed: {}", e)))?;
        
        let seed_array: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (keypair, _) = crate::core::wallet::derive_keypair_from_seed(
            &seed_array,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption("Failed to derive keypair".to_string()))?;

        Ok(keypair.to_bytes().to_vec())
    }

    /// Sign a transaction using the appropriate wallet (Internal or Backpack)
    /// 
    /// This method handles signing based on the wallet type:
    /// - **Internal wallet**: Uses secure in-memory signing with Zeroizing
    /// - **Backpack wallet**: Delegates to Backpack's signTransaction API
    /// 
    /// # Parameters
    /// * `transaction` - Mutable reference to the transaction to sign
    /// 
    /// # Returns
    /// Ok(()) on success, SessionError on failure
    async fn sign_transaction(&self, transaction: &mut Transaction) -> Result<(), SessionError> {
        match self.wallet_type {
            WalletType::Internal => {
                // Internal wallet: sign with keypair from seed
                self.sign_transaction_internal(transaction)
            },
            WalletType::Backpack => {
                // Backpack wallet: sign via JavaScript bridge
                self.sign_transaction_backpack(transaction).await
            }
        }
    }

    /// Sign a transaction using the internal wallet (secure in-memory signing)
    fn sign_transaction_internal(&self, transaction: &mut Transaction) -> Result<(), SessionError> {
        // Get seed (wrapped in Zeroizing for automatic cleanup)
        let seed = Zeroizing::new(self.get_seed()?);
        
        // Decode seed bytes (also wrapped in Zeroizing)
        let seed_bytes = Zeroizing::new(
            hex::decode(seed.as_str())
                .map_err(|e| SessionError::Encryption(format!("Failed to decode seed: {}", e)))?
        );
        
        // Create seed array for keypair derivation
        let mut seed_array = [0u8; 64];
        seed_array.copy_from_slice(&seed_bytes);
        
        // Derive keypair from seed
        let (keypair, _) = crate::core::wallet::derive_keypair_from_seed(
            &seed_array,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption(format!("Failed to derive keypair: {:?}", e)))?;
        
        // Sign the transaction
        transaction.sign(&[&keypair], transaction.message.recent_blockhash);
        
        // Explicitly clear the seed array
        seed_array.zeroize();
        
        // Note: keypair will be dropped here, seed and seed_bytes are automatically zeroized
        log::debug!("Transaction signed successfully with internal wallet");
        
        Ok(())
    }

    /// Sign a transaction using Backpack wallet
    async fn sign_transaction_backpack(&self, transaction: &mut Transaction) -> Result<(), SessionError> {
        // Serialize transaction to base64
        let tx_bytes = bincode::serialize(&transaction)
            .map_err(|e| SessionError::InvalidData(format!("Failed to serialize transaction: {}", e)))?;
        let tx_base64 = base64::encode(&tx_bytes);
        
        log::debug!("Requesting signature from Backpack wallet...");
        
        // Call Backpack to sign the transaction
        let signed_tx_base64 = BackpackWallet::sign_transaction(&tx_base64).await?;
        
        // Decode the signed transaction
        let signed_tx_bytes = base64::decode(&signed_tx_base64)
            .map_err(|e| SessionError::InvalidData(format!("Failed to decode signed transaction: {}", e)))?;
        
        let signed_transaction: Transaction = bincode::deserialize(&signed_tx_bytes)
            .map_err(|e| SessionError::InvalidData(format!("Failed to deserialize signed transaction: {}", e)))?;
        
        // Update the original transaction with the signed version
        *transaction = signed_transaction;
        
        log::debug!("Transaction signed successfully with Backpack wallet");
        
        Ok(())
    }

    /// Mint tokens using memo
    /// 
    /// This method follows a secure pattern:
    /// 1. RPC builds unsigned transaction
    /// 2. Session signs transaction (private key stays in Session)
    /// 3. RPC sends signed transaction
    /// 
    /// # Parameters
    /// * `memo` - The memo text (must be 69-800 bytes)
    /// 
    /// # Returns
    /// Transaction signature on success
    pub async fn mint(&mut self, memo: &str) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let rpc = RpcConnection::new();
        
        // Get user's public key
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        // Step 1: RPC builds unsigned transaction
        log::info!("Building mint transaction...");
        let mut transaction = rpc.build_mint_transaction(&pubkey, memo).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to build transaction: {}", e)))?;
        
        // Step 2: Session signs transaction (private key never leaves Session)
        log::info!("Signing transaction in Session...");
        self.sign_transaction(&mut transaction).await?;
        
        // Step 3: RPC sends signed transaction
        log::info!("Sending signed transaction...");
        let tx_hash = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to send transaction: {}", e)))?;
        
        log::info!("Mint transaction sent successfully: {}", tx_hash);
        self.balance_update_needed = true;
        
        Ok(tx_hash)
    }

    // check if user has profile
    pub fn has_user_profile(&self) -> bool {
        self.user_profile.is_some()
    }

    // balance related methods
    pub fn get_sol_balance(&self) -> f64 {
        self.sol_balance
    }

    pub fn get_token_balance(&self) -> f64 {
        self.token_balance
    }

    pub fn set_balances(&mut self, sol_balance: f64, token_balance: f64) {
        self.sol_balance = sol_balance;
        self.token_balance = token_balance;
        self.balance_update_needed = false;
    }

    pub fn mark_balance_update_needed(&mut self) {
        self.balance_update_needed = true;
    }

    pub fn is_balance_update_needed(&self) -> bool {
        self.balance_update_needed
    }

    // fetch and update balances
    pub async fn fetch_and_update_balances(&mut self) -> Result<(), SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let pubkey = self.get_public_key()?;
        let rpc = RpcConnection::new();
        
        // Get token balance using dynamic token mint
        let token_mint = MintConfig::get_token_mint()
            .map_err(|e| SessionError::InvalidData(format!("Failed to get token mint: {}", e)))?;
        match rpc.get_token_balance(&pubkey, &token_mint.to_string()).await {
            Ok(token_result) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&token_result) {
                    if let Some(accounts) = json.get("value").and_then(|v| v.as_array()) {
                        if let Some(first_account) = accounts.first() {
                            if let Some(amount) = first_account
                                .get("account")
                                .and_then(|a| a.get("data"))
                                .and_then(|d| d.get("parsed"))
                                .and_then(|p| p.get("info"))
                                .and_then(|i| i.get("tokenAmount"))
                                .and_then(|t| t.get("uiAmount"))
                                .and_then(|a| a.as_f64())
                            {
                                self.token_balance = amount;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to get token balance: {}", e);
            }
        }

        // get SOL balance
        match rpc.get_balance(&pubkey).await {
            Ok(balance_result) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&balance_result) {
                    if let Some(lamports) = json.get("value").and_then(|v| v.as_u64()) {
                        let sol = lamports as f64 / 1_000_000_000.0;
                        self.sol_balance = sol;
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to get SOL balance: {}", e);
            }
        }

        self.balance_update_needed = false;
        Ok(())
    }

    // check if expiration is enabled
    pub fn has_expiration(&self) -> bool {
        self.config.timeout_minutes.is_some()
    }

    // get expiration time setting
    pub fn get_timeout_minutes(&self) -> Option<u32> {
        self.config.timeout_minutes
    }

    // set expiration time (None = never expire)
    pub fn set_timeout(&mut self, timeout_minutes: Option<u32>) {
        self.config.timeout_minutes = timeout_minutes;
        // reset start time
        self.start_time = Date::now();
    }

    // get session remaining time
    pub fn get_remaining_time(&self) -> Option<f64> {
        match self.config.timeout_minutes {
            None => None, // never expire
            Some(timeout_minutes) => {
                let current_time = Date::now();
                let elapsed_minutes = (current_time - self.start_time) / (60.0 * 1000.0);
                let remaining = timeout_minutes as f64 - elapsed_minutes;
                Some(remaining.max(0.0))
            }
        }
    }

    /// Send chat message to group - internal handle all key operations
    pub async fn send_chat_message(
        &mut self, 
        group_id: u64, 
        message: &str,
        receiver: Option<String>,
        reply_to_sig: Option<String>
    ) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let rpc = RpcConnection::new();
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        log::info!("Building send chat message transaction...");
        let mut transaction = rpc.build_send_chat_message_transaction(&pubkey, group_id, message, receiver, reply_to_sig).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to build transaction: {}", e)))?;
        
        log::info!("Signing transaction in Session...");
        self.sign_transaction(&mut transaction).await?;
        
        log::info!("Sending signed transaction...");
        let tx_hash = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to send transaction: {}", e)))?;
        
        log::info!("Chat message sent successfully: {}", tx_hash);
        self.balance_update_needed = true;
        
        Ok(tx_hash)
    }

    /// Send a chat message to a group with timeout
    /// Note: Timeout handling is currently simplified in the new architecture
    pub async fn send_chat_message_with_timeout(
        &mut self, 
        group_id: u64, 
        message: &str,
        receiver: Option<String>,
        reply_to_sig: Option<String>,
        timeout_ms: Option<u32>
    ) -> Result<String, SessionError> {
        if timeout_ms.is_some() {
            log::warn!("Timeout parameter is currently not supported in the new architecture");
        }
        // Use the standard send_chat_message method
        self.send_chat_message(group_id, message, receiver, reply_to_sig).await
    }

    /// Create a new chat group - internal handle all key operations
    pub async fn create_chat_group(
        &mut self,
        name: &str,
        description: &str,
        image: &str,
        tags: Vec<String>,
        min_memo_interval: Option<i64>,
        burn_amount: u64,
    ) -> Result<(String, u64), SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        log::info!("Session: Creating chat group '{}' with {} tokens", name, burn_amount / 1_000_000);

        let rpc = RpcConnection::new();
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        log::info!("Building create chat group transaction...");
        let (mut transaction, group_id) = rpc.build_create_chat_group_transaction(
            &pubkey, name, description, image, tags, min_memo_interval, burn_amount
        ).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to build transaction: {}", e)))?;
        
        log::info!("Signing transaction in Session...");
        self.sign_transaction(&mut transaction).await?;
        
        log::info!("Sending signed transaction...");
        let tx_hash = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to send transaction: {}", e)))?;
        
        log::info!("Session: Chat group '{}' created successfully with ID {}", name, group_id);
        self.mark_balance_update_needed();
        
        Ok((tx_hash, group_id))
    }

    /// Burn tokens for a chat group
    /// 
    /// # Parameters
    /// * `group_id` - The ID of the chat group to burn tokens for
    /// * `amount` - Amount of MEMO tokens to burn (in token units, not lamports)
    /// * `message` - Optional burn message (max 512 characters)
    /// 
    /// # Returns
    /// Result containing transaction signature
    pub async fn burn_tokens_for_group(
        &mut self,
        group_id: u64,
        amount: u64,
        message: &str,
    ) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let rpc = crate::core::rpc_base::RpcConnection::new();
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        // Convert amount from tokens to lamports
        let amount_lamports = amount * 1_000_000;
        
        log::info!("Building burn tokens for group transaction...");
        let mut transaction = rpc.build_burn_tokens_for_group_transaction(&pubkey, group_id, amount_lamports, message).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to build transaction: {}", e)))?;
        
        log::info!("Signing transaction in Session...");
        self.sign_transaction(&mut transaction).await?;
        
        log::info!("Sending signed transaction...");
        let signature = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to send transaction: {}", e)))?;
        
        log::info!("Tokens burned successfully for group {}", group_id);
        
        // Update balances after successful burn
        match self.fetch_and_update_balances().await {
            Ok(()) => {
                log::info!("Successfully updated balances after burning tokens for group");
            },
            Err(e) => {
                log::error!("Failed to update balances after burning tokens for group: {}", e);
                self.mark_balance_update_needed();
            }
        }

        Ok(signature)
    }

    /// Create a new project - internal handle all key operations
    /// 
    /// # Parameters
    /// * `name` - Project name (1-64 characters)
    /// * `description` - Project description (max 256 characters)
    /// * `image` - Project image URL (max 256 characters)
    /// * `website` - Project website URL (max 128 characters)
    /// * `tags` - Project tags (max 4 tags, each max 32 characters)
    /// * `burn_amount` - Amount of MEMO tokens to burn (in token units, not lamports)
    /// 
    /// # Returns
    /// Result containing transaction signature and project ID
    pub async fn create_project(
        &mut self,
        name: &str,
        description: &str,
        image: &str,
        website: &str,
        tags: Vec<String>,
        burn_amount: u64,
    ) -> Result<(String, u64), SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        log::info!("Session: Creating project '{}' with {} tokens", name, burn_amount);

        let rpc = crate::core::rpc_base::RpcConnection::new();
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        // Convert amount from tokens to lamports
        let burn_amount_lamports = burn_amount * 1_000_000;
        
        log::info!("Building create project transaction...");
        let (mut transaction, project_id) = rpc.build_create_project_transaction(
            &pubkey, name, description, image, website, tags, burn_amount_lamports
        ).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to build transaction: {}", e)))?;
        
        log::info!("Signing transaction in Session...");
        self.sign_transaction(&mut transaction).await?;
        
        log::info!("Sending signed transaction...");
        let tx_hash = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to send transaction: {}", e)))?;
        
        log::info!("Session: Project '{}' created successfully with ID {}", name, project_id);
        self.mark_balance_update_needed();
        
        Ok((tx_hash, project_id))
    }

    /// Update an existing project
    /// 
    /// # Parameters
    /// * `project_id` - The ID of the project to update
    /// * `name` - New project name (optional, 1-64 characters)
    /// * `description` - New project description (optional, max 256 characters)
    /// * `image` - New project image URL (optional, max 256 characters)
    /// * `website` - New project website URL (optional, max 128 characters)
    /// * `tags` - New project tags (optional, max 4 tags, each max 32 characters)
    /// * `burn_amount` - Amount of MEMO tokens to burn (in token units, not lamports)
    /// 
    /// # Returns
    /// Result containing transaction signature
    pub async fn update_project(
        &mut self,
        project_id: u64,
        name: Option<String>,
        description: Option<String>,
        image: Option<String>,
        website: Option<String>,
        tags: Option<Vec<String>>,
        burn_amount: u64,
    ) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        log::info!("Session: Updating project {} with {} tokens", project_id, burn_amount);

        let rpc = crate::core::rpc_base::RpcConnection::new();
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        // Convert amount from tokens to lamports
        let burn_amount_lamports = burn_amount * 1_000_000;
        
        log::info!("Building update project transaction...");
        let mut transaction = rpc.build_update_project_transaction(
            &pubkey, project_id, name, description, image, website, tags, burn_amount_lamports
        ).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to build transaction: {}", e)))?;
        
        log::info!("Signing transaction in Session...");
        self.sign_transaction(&mut transaction).await?;
        
        log::info!("Sending signed transaction...");
        let signature = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to send transaction: {}", e)))?;
        
        log::info!("Session: Project {} updated successfully", project_id);
        self.mark_balance_update_needed();
        
        Ok(signature)
    }

    /// Burn tokens for a project
    /// 
    /// # Parameters
    /// * `project_id` - The ID of the project to burn tokens for
    /// * `amount` - Amount of MEMO tokens to burn (in token units, not lamports)
    /// * `message` - Optional burn message (max 696 characters)
    /// 
    /// # Returns
    /// Result containing transaction signature
    pub async fn burn_tokens_for_project(
        &mut self,
        project_id: u64,
        amount: u64,
        message: &str,
    ) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let rpc = crate::core::rpc_base::RpcConnection::new();
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        // Convert amount from tokens to lamports
        let amount_lamports = amount * 1_000_000;
        
        log::info!("Building burn tokens for project transaction...");
        let mut transaction = rpc.build_burn_tokens_for_project_transaction(&pubkey, project_id, amount_lamports, message).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to build transaction: {}", e)))?;
        
        log::info!("Signing transaction in Session...");
        self.sign_transaction(&mut transaction).await?;
        
        log::info!("Sending signed transaction...");
        let signature = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to send transaction: {}", e)))?;
        
        log::info!("Tokens burned successfully for project {}", project_id);
        
        // Update balances after successful burn
        match self.fetch_and_update_balances().await {
            Ok(()) => {
                log::info!("Successfully updated balances after burning tokens for project");
            },
            Err(e) => {
                log::error!("Failed to update balances after burning tokens for project: {}", e);
                self.mark_balance_update_needed();
            }
        }

        Ok(signature)
    }

    /// Get information for a specific project (doesn't require authentication)
    /// 
    /// # Parameters
    /// * `project_id` - The ID of the project to fetch
    /// 
    /// # Returns
    /// Project information if it exists
    pub async fn get_project_info(&self, project_id: u64) -> Result<ProjectInfo, SessionError> {
        let rpc = crate::core::rpc_base::RpcConnection::new();
        rpc.get_project_info(project_id).await
            .map_err(|e| SessionError::InvalidData(format!("Get project info failed: {}", e)))
    }

    /// Get comprehensive statistics for all projects (doesn't require authentication)
    /// 
    /// # Returns
    /// Complete statistics including all project information
    pub async fn get_all_project_statistics(&self) -> Result<ProjectStatistics, SessionError> {
        let rpc = crate::core::rpc_base::RpcConnection::new();
        rpc.get_all_project_statistics().await
            .map_err(|e| SessionError::InvalidData(format!("Get project statistics failed: {}", e)))
    }

    /// Get project burn leaderboard (doesn't require authentication)
    /// 
    /// # Returns
    /// Project burn leaderboard data, including the top 100 projects
    pub async fn get_project_burn_leaderboard(&self) -> Result<ProjectBurnLeaderboardResponse, SessionError> {
        let rpc = crate::core::rpc_base::RpcConnection::new();
        rpc.get_project_burn_leaderboard().await
            .map_err(|e| SessionError::InvalidData(format!("Get project burn leaderboard failed: {}", e)))
    }

    /// Get the rank of a specific project in the burn leaderboard (doesn't require authentication)
    /// 
    /// # Parameters
    /// * `project_id` - Project ID
    /// 
    /// # Returns
    /// Rank (1-100), return None if the project is not in the leaderboard
    pub async fn get_project_burn_rank(&self, project_id: u64) -> Result<Option<u8>, SessionError> {
        let rpc = crate::core::rpc_base::RpcConnection::new();
        rpc.get_project_burn_rank(project_id).await
            .map_err(|e| SessionError::InvalidData(format!("Get project burn rank failed: {}", e)))
    }

    /// Check if a specific project exists (doesn't require authentication)
    /// 
    /// # Parameters
    /// * `project_id` - The ID of the project to check
    /// 
    /// # Returns
    /// True if the project exists, false otherwise
    pub async fn project_exists(&self, project_id: u64) -> Result<bool, SessionError> {
        let rpc = crate::core::rpc_base::RpcConnection::new();
        rpc.project_exists(project_id).await
            .map_err(|e| SessionError::InvalidData(format!("Check project exists failed: {}", e)))
    }

    /// Get the total number of projects that have been created (doesn't require authentication)
    /// 
    /// # Returns
    /// The total number of projects from the global counter
    pub async fn get_total_projects(&self) -> Result<u64, SessionError> {
        let rpc = crate::core::rpc_base::RpcConnection::new();
        rpc.get_total_projects().await
            .map_err(|e| SessionError::InvalidData(format!("Get total projects failed: {}", e)))
    }

    // fetch and cache user burn stats
    pub async fn fetch_and_cache_user_burn_stats(&mut self) -> Result<Option<UserGlobalBurnStats>, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let pubkey = self.get_public_key()?;
        let rpc = RpcConnection::new();

        match rpc.get_user_global_burn_stats(&pubkey).await {
            Ok(Some(stats)) => {
                log::info!("Successfully fetched and cached user burn stats");
                self.user_burn_stats = Some(stats.clone());
                Ok(Some(stats))
            },
            Ok(None) => {
                log::info!("User burn stats not found for pubkey: {}", pubkey);
                self.user_burn_stats = None;
                Ok(None)
            },
            Err(e) => {
                log::error!("Failed to fetch user burn stats: {}", e);
                Err(SessionError::InvalidData(format!("RPC error: {}", e)))
            }
        }
    }

    // initialize user global burn stats
    pub async fn initialize_user_burn_stats(&mut self) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let rpc = RpcConnection::new();
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        // Step 1: Build unsigned transaction
        let mut transaction = rpc.build_initialize_burn_stats_transaction(&pubkey).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to build initialize transaction: {}", e)))?;
        
        // Step 2: Sign in Session
        self.sign_transaction(&mut transaction).await?;
        
        // Step 3: Send signed transaction
        let tx_hash = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to send initialize transaction: {}", e)))?;
        
        log::info!("User burn stats initialized successfully: {}", tx_hash);
        let _ = self.fetch_and_cache_user_burn_stats().await;
        Ok(tx_hash)
    }

    // burn tokens using memo-burn contract
    pub async fn burn_tokens(&mut self, amount: u64, message: &str) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        // Check if burn stats are initialized, if not, initialize them first
        if self.user_burn_stats.is_none() {
            log::info!("Burn stats not initialized, initializing first...");
            self.initialize_user_burn_stats().await?;
        }

        let rpc = RpcConnection::new();
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        // Step 1: Build unsigned transaction
        let mut transaction = rpc.build_burn_transaction(&pubkey, amount, message).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to build burn transaction: {}", e)))?;
        
        // Step 2: Sign in Session
        self.sign_transaction(&mut transaction).await?;
        
        // Step 3: Send signed transaction
        let tx_hash = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to send burn transaction: {}", e)))?;
        
        log::info!("Burn transaction sent: {}", tx_hash);
        self.balance_update_needed = true;
        let _ = self.fetch_and_cache_user_burn_stats().await;
        Ok(tx_hash)
    }

    // check if user has burn stats initialized
    pub fn has_burn_stats_initialized(&self) -> bool {
        self.user_burn_stats.is_some()
    }

    // get user burn stats
    pub fn get_user_burn_stats(&self) -> Option<UserGlobalBurnStats> {
        self.user_burn_stats.clone()
    }

    pub fn set_user_burn_stats(&mut self, stats: Option<UserGlobalBurnStats>) {
        self.user_burn_stats = stats;
    }

    /// Transfer native tokens (XNT/SOL) to another address
    /// 
    /// # Parameters
    /// * `to_address` - Recipient's address
    /// * `amount_lamports` - Amount to transfer in lamports
    /// 
    /// # Returns
    /// Transaction signature on success
    pub async fn transfer_native(
        &mut self,
        to_address: &str,
        amount_lamports: u64,
    ) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let rpc = RpcConnection::new();
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        log::info!("Building native transfer transaction...");
        let mut transaction = rpc.build_native_transfer_transaction(&pubkey, to_address, amount_lamports).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to build transaction: {}", e)))?;
        
        log::info!("Signing transaction in Session...");
        self.sign_transaction(&mut transaction).await?;
        
        log::info!("Sending signed transaction...");
        let tx_hash = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to send transaction: {}", e)))?;
        
        log::info!("Native transfer successful: {}", tx_hash);
        self.balance_update_needed = true;
        
        Ok(tx_hash)
    }

    /// Transfer SPL tokens (MEMO) to another address
    /// 
    /// # Parameters
    /// * `to_address` - Recipient's address
    /// * `amount` - Amount to transfer in token units (with decimals)
    /// 
    /// # Returns
    /// Transaction signature on success
    pub async fn transfer_token(
        &mut self,
        to_address: &str,
        amount: u64,
    ) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let rpc = RpcConnection::new();
        let pubkey_str = self.get_public_key()?;
        let pubkey = Pubkey::from_str(&pubkey_str)
            .map_err(|e| SessionError::InvalidData(format!("Invalid pubkey: {}", e)))?;
        
        log::info!("Building token transfer transaction...");
        let mut transaction = rpc.build_token_transfer_transaction(&pubkey, to_address, amount).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to build transaction: {}", e)))?;
        
        log::info!("Signing transaction in Session...");
        self.sign_transaction(&mut transaction).await?;
        
        log::info!("Sending signed transaction...");
        let tx_hash = rpc.send_signed_transaction(&transaction).await
            .map_err(|e| SessionError::InvalidData(format!("Failed to send transaction: {}", e)))?;
        
        log::info!("Token transfer successful: {}", tx_hash);
        self.balance_update_needed = true;
        
        Ok(tx_hash)
    }
}

// implement zeroize for Session to ensure sensitive data is cleared
impl Zeroize for Session {
    fn zeroize(&mut self) {
        self.clear();
    }
}

// implement drop for Session to ensure sensitive data is cleared
impl Drop for Session {
    fn drop(&mut self) {
        self.clear();
    }
}

// Legacy parse function for compatibility (now uses new profile system)
pub fn parse_user_profile(account_data: &str) -> Result<UserProfile, SessionError> {
    log::info!("Starting to parse user profile from account data using new format");
    
    match parse_user_profile_new(account_data) {
        Ok(profile) => {
            log::info!("Successfully parsed new format user profile");
            Ok(profile)
        },
        Err(e) => {
            log::error!("Failed to parse new format user profile: {}", e);
            Err(SessionError::ProfileError(format!("Parse error: {}", e)))
        }
    }
} 
