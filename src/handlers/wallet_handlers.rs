use dioxus::prelude::*;
use crate::wallet;
use crate::storage;
use crate::session;
use crate::services::balance_service::fetch_balance;

// Generate a new wallet and show password modal
pub fn generate_new_wallet(
    mut mnemonic: Signal<String>,
    mut password: Signal<String>,
    mut confirm_password: Signal<String>,
    mut show_password_modal: Signal<bool>,
) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        // Generate a new mnemonic
        let new_mnemonic = wallet::generate_mnemonic();
        mnemonic.set(new_mnemonic);
        
        // Show password input modal
        password.set(String::new());
        confirm_password.set(String::new());
        show_password_modal.set(true);
        
        log::info!("New Wallet button clicked, showing password modal");
    }
}

// Confirm password and show mnemonic
pub fn confirm_password_and_show_mnemonic(
    password: Signal<String>,
    confirm_password: Signal<String>,
    mut error_message: Signal<String>,
    mut show_password_modal: Signal<bool>,
    mut show_modal: Signal<bool>,
    mut wallet_saved: Signal<bool>,
) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        let pwd = password.read().clone();
        let confirm_pwd = confirm_password.read().clone();
        
        // Validate password
        if pwd.is_empty() {
            error_message.set("Password cannot be empty".to_string());
            return;
        }
        
        if pwd != confirm_pwd {
            error_message.set("Passwords do not match".to_string());
            return;
        }
        
        // Password validation passed, show mnemonic
        show_password_modal.set(false);
        show_modal.set(true);
        wallet_saved.set(false);
        error_message.set(String::new());
        
        log::info!("Password confirmed, showing mnemonic");
    }
}

// Show import wallet modal
pub fn show_import_wallet_modal(
    mut import_mnemonic: Signal<String>,
    mut error_message: Signal<String>,
    mut show_import_modal: Signal<bool>,
) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        // Reset import states
        import_mnemonic.set(String::new());
        error_message.set(String::new());
        show_import_modal.set(true);
        
        log::info!("Import Wallet button clicked, showing import modal");
    }
}

// Confirm import mnemonic
pub fn confirm_import_mnemonic(
    import_mnemonic: Signal<String>,
    mut error_message: Signal<String>,
    mut show_import_modal: Signal<bool>,
    mut import_password: Signal<String>,
    mut import_confirm_password: Signal<String>,
    mut show_import_password_modal: Signal<bool>,
) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        let mnemonic_str = import_mnemonic.read().clone();
        
        // Validate mnemonic
        if mnemonic_str.is_empty() {
            error_message.set("Recovery phrase cannot be empty".to_string());
            return;
        }
        
        // Validate mnemonic format
        if let Err(err) = wallet::validate_mnemonic(&mnemonic_str) {
            error_message.set(format!("Invalid recovery phrase: {}", err));
            return;
        }
        
        // Mnemonic validation passed, show password modal
        show_import_modal.set(false);
        import_password.set(String::new());
        import_confirm_password.set(String::new());
        show_import_password_modal.set(true);
        error_message.set(String::new());
        
        log::info!("Import mnemonic validated, showing password modal");
    }
}

// Confirm import password
pub fn confirm_import_password(
    import_mnemonic: Signal<String>,
    import_password: Signal<String>,
    import_confirm_password: Signal<String>,
    mut error_message: Signal<String>,
    mut is_importing: Signal<bool>,
    mut wallet_address: Signal<String>,
    mut wallet_saved: Signal<bool>,
    mut show_import_password_modal: Signal<bool>,
    mut wallet_balance: Signal<Option<f64>>,
    mut is_loading_balance: Signal<bool>,
    mut session_active: Signal<bool>,
) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        is_importing.set(true);
        let mnemonic_str = import_mnemonic.read().clone();
        let pwd = import_password.read().clone();
        let confirm_pwd = import_confirm_password.read().clone();
        
        // Validate password
        if pwd.is_empty() {
            error_message.set("Password cannot be empty".to_string());
            is_importing.set(false);
            return;
        }
        
        if pwd != confirm_pwd {
            error_message.set("Passwords do not match".to_string());
            is_importing.set(false);
            return;
        }
        
        // Save imported wallet
        match storage::create_and_save_wallet(mnemonic_str, &pwd) {
            Ok(_) => {
                // Get address from session
                if let Some(address) = wallet::get_wallet_address() {
                    let address_clone = address.clone();
                    wallet_address.set(address);
                    wallet_saved.set(true);
                    error_message.set(String::new());
                    show_import_password_modal.set(false);
                    
                    // Update session status
                    session_active.set(session::is_session_active());
                    
                    log::info!("Wallet imported successfully with address: {}", address_clone);
                    
                    // Fetch wallet balance for the imported wallet
                    fetch_balance(address_clone, wallet_balance.clone(), is_loading_balance.clone());
                } else {
                    error_message.set("Failed to get wallet address from session".to_string());
                    log::error!("Failed to get wallet address from session");
                }
            },
            Err(err) => {
                error_message.set(format!("Failed to import wallet: {}", err));
                log::error!("Failed to import wallet: {}", err);
            }
        }
        
        is_importing.set(false);
    }
}

// Show decrypt dialog
pub fn show_decrypt_dialog(
    mut decryption_password: Signal<String>,
    mut show_decrypt_modal: Signal<bool>,
    mut is_decrypting: Signal<bool>,
    mut error_message: Signal<String>,
) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        decryption_password.set(String::new());
        show_decrypt_modal.set(true);
        is_decrypting.set(false);
        error_message.set(String::new());
    }
}

// Decrypt mnemonic
pub fn decrypt_mnemonic(
    mut is_decrypting: Signal<bool>,
    decryption_password: Signal<String>,
    mut error_message: Signal<String>,
    mut mnemonic: Signal<String>,
    mut show_modal: Signal<bool>,
    mut show_decrypt_modal: Signal<bool>,
    mut session_active: Signal<bool>,
    mut wallet_address: Signal<String>,
) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        is_decrypting.set(true);
        let pwd = decryption_password.read().clone();
        
        if pwd.is_empty() {
            error_message.set("Password cannot be empty".to_string());
            is_decrypting.set(false);
            return;
        }
        
        match storage::load_wallet() {
            Ok(Some(wallet)) => {
                match storage::decrypt_mnemonic(&wallet, &pwd) {
                    Ok(decrypted_mnemonic) => {
                        mnemonic.set(decrypted_mnemonic);
                        show_modal.set(true);
                        show_decrypt_modal.set(false);
                        error_message.set(String::new());
                        log::info!("Mnemonic decrypted successfully");
                        
                        // Update session status for UI
                        session_active.set(session::is_session_active());
                        
                        // Update wallet address from session
                        if let Some(address) = wallet::get_wallet_address() {
                            let address_clone = address.clone();
                            wallet_address.set(address);
                            log::info!("Updated wallet address from session: {}", address_clone);
                        }
                    },
                    Err(err) => {
                        error_message.set(format!("Failed to decrypt mnemonic: {}", err));
                        log::error!("Failed to decrypt mnemonic: {}", err);
                    }
                }
            },
            Ok(None) => {
                error_message.set("No wallet found".to_string());
                log::error!("No wallet found");
            },
            Err(err) => {
                error_message.set(format!("Failed to load wallet: {}", err));
                log::error!("Failed to load wallet: {}", err);
            }
        }
        
        is_decrypting.set(false);
    }
}

// Clear wallet
pub fn clear_wallet(
    mut wallet_address: Signal<String>,
    mut wallet_saved: Signal<bool>,
    mut mnemonic: Signal<String>,
    mut error_message: Signal<String>,
    mut wallet_balance: Signal<Option<f64>>,
    mut session_active: Signal<bool>,
) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(local_storage)) = window.local_storage() {
                    let _ = local_storage.remove_item("wallet_data");
                    log::info!("Wallet data cleared from localStorage");
                    
                    // Reset state
                    wallet_address.set(String::new());
                    wallet_saved.set(false);
                    mnemonic.set(String::new());
                    error_message.set(String::new());
                    wallet_balance.set(None);
                    
                    // Clear the session
                    session::clear_session();
                    
                    // Update session status
                    session_active.set(false);
                    
                    log::info!("Wallet session cleared");
                }
            }
        }
    }
}

// Refresh balance
pub fn refresh_balance(
    wallet_address: Signal<String>,
    mut wallet_balance: Signal<Option<f64>>,
    mut is_loading_balance: Signal<bool>,
) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        let address = wallet_address.read().clone();
        if !address.is_empty() {
            fetch_balance(address, wallet_balance.clone(), is_loading_balance.clone());
        }
    }
}

// Handle session password submission
pub fn handle_session_password_submit(
    mut session_password_error: Signal<String>,
    mut is_decrypting: Signal<bool>,
    mut show_session_password_modal: Signal<bool>,
    mut session_active: Signal<bool>,
    mut wallet_address: Signal<String>,
    mut wallet_balance: Signal<Option<f64>>,
    mut is_loading_balance: Signal<bool>,
) -> impl FnMut(String) {
    move |pwd: String| {
        session_password_error.set(String::new());
        is_decrypting.set(true);
        
        // Load wallet data
        match storage::load_wallet() {
            Ok(Some(wallet)) => {
                // Try to decrypt the mnemonic
                match storage::decrypt_mnemonic(&wallet, &pwd) {
                    Ok(_) => {
                        // Successfully decrypted and stored in session
                        show_session_password_modal.set(false);
                        is_decrypting.set(false);
                        
                        // Update session status
                        session_active.set(session::is_session_active());
                        
                        // Get the wallet address
                        if let Some(address) = wallet::get_wallet_address() {
                            let address_clone = address.clone();
                            wallet_address.set(address);
                            log::info!("Unlocked wallet with address: {}", address_clone);
                            
                            // Fetch wallet balance
                            fetch_balance(address_clone, wallet_balance.clone(), is_loading_balance.clone());
                        }
                    },
                    Err(err) => {
                        is_decrypting.set(false);
                        session_password_error.set(format!("Failed to unlock wallet: {}", err));
                        log::error!("Failed to decrypt mnemonic: {}", err);
                    }
                }
            },
            Ok(None) => {
                is_decrypting.set(false);
                session_password_error.set("No wallet found".to_string());
                log::error!("No wallet found when trying to decrypt");
            },
            Err(err) => {
                is_decrypting.set(false);
                session_password_error.set(format!("Failed to load wallet: {}", err));
                log::error!("Failed to load wallet: {}", err);
            }
        }
    }
}

// Handle session password cancel
pub fn handle_session_password_cancel(
    mut show_session_password_modal: Signal<bool>,
    mut wallet_saved: Signal<bool>,
) -> impl FnMut(()) {
    move |_| {
        show_session_password_modal.set(false);
        // Set wallet saved to true to prevent showing the mnemonic again
        wallet_saved.set(true);
    }
} 