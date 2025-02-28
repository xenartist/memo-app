use dioxus::prelude::*;
use crate::wallet;
use crate::storage;
use crate::components::{MnemonicModal, WalletAddressDisplay, PixelCanvas, PasswordModal};
use crate::config;
use crate::session;
use crate::handlers::{
    generate_new_wallet, confirm_password_and_show_mnemonic, show_import_wallet_modal,
    confirm_import_mnemonic, confirm_import_password, show_decrypt_dialog, decrypt_mnemonic,
    clear_wallet, refresh_balance, handle_session_password_submit, handle_session_password_cancel,
    show_send_transaction_modal, prepare_transaction, confirm_transaction_with_password,
    create_sign_and_send_transaction, close_send_modal, close_transaction_password_modal,
    show_mint_modal, close_mint_modal, process_mint
};
use crate::services::fetch_balance;
use std::rc::Rc;
use std::cell::RefCell;

// Home page component
pub fn Home() -> Element {
    // State for modal visibility and mnemonic
    let mut show_modal = use_signal(|| false);
    let mut mnemonic = use_signal(|| String::new());
    let mut wallet_saved = use_signal(|| false);
    let mut error_message = use_signal(|| String::new());
    let mut wallet_address = use_signal(|| String::new());
    let mut wallet_balance = use_signal(|| None::<f64>);
    let mut is_loading_balance = use_signal(|| false);
    
    // Password related states
    let mut password = use_signal(|| String::new());
    let mut confirm_password = use_signal(|| String::new());
    let mut show_password_modal = use_signal(|| false);
    let mut is_decrypting = use_signal(|| false);
    let mut decryption_password = use_signal(|| String::new());
    let mut show_decrypt_modal = use_signal(|| false);
    
    // Session check modal
    let mut show_session_password_modal = use_signal(|| false);
    let mut session_password_error = use_signal(|| String::new());
    
    // Import wallet related states
    let mut show_import_modal = use_signal(|| false);
    let mut import_mnemonic = use_signal(|| String::new());
    let mut import_password = use_signal(|| String::new());
    let mut import_confirm_password = use_signal(|| String::new());
    let mut show_import_password_modal = use_signal(|| false);
    let mut is_importing = use_signal(|| false);
    
    // Transaction related states
    let mut show_send_modal = use_signal(|| false);
    let mut recipient_address = use_signal(|| String::new());
    let mut send_amount = use_signal(|| String::new());
    let mut transaction_memo = use_signal(|| String::new());
    let mut is_sending = use_signal(|| false);
    let mut transaction_password = use_signal(|| String::new());
    let mut show_transaction_password_modal = use_signal(|| false);
    let mut prepared_transaction = use_signal(|| None::<wallet::Transaction>);
    let mut session_active = use_signal(|| session::is_session_active());
    
    // Mint related states
    let mut show_mint_modal = use_signal(|| false);
    let mut is_minting = use_signal(|| false);
    let mut memo_name = use_signal(|| String::new());
    let mut memo_description = use_signal(|| String::new());
    
    // Hex string for the pixel canvas
    let pixel_hex = "00003FFF000000001FFFFE00000007FFFFF8000001FFFFFFE000003FFFFFFF000007FFFFFFF80000FFFFFFFFC0001FFFFFFFFE0003FFFFFFFFF0007FFFFFFFFF800FFFFFFFFFFC01FFCFFFFCFFE01FFCFFFF8FFE03FFE7FFF9FFF03FFF3FFF3FFF07FFF9FFE7FFF87FFF9FFCFFFF87FFFCFFCFFFF8FFFFE7F9FFFFCFFFFF3F3FFFFCFFFFF1E7FFFFCFFFFF9E7FFFFCFFFFFCCFFFFFCFFFFFE1FFFFFCFFFFFE3FFFFFCFFFFFF3FFFFFCFFFFFE1FFFFFCFFFFFCCFFFFFCFFFFF9C7FFFFCFFFFF9E7FFFFCFFFFF3F3FFFFCFFFFE7F9FFFFC7FFFCFF8FFFF87FFFCFFCFFFF87FFF9FFE7FFF83FFF3FFF3FFF03FFE7FFF1FFF01FFC7FFF9FFE01FFCFFFFCFFE00FFFFFFFFFFC007FFFFFFFFF8003FFFFFFFFF0001FFFFFFFFE0000FFFFFFFFC00007FFFFFFF800003FFFFFFF000001FFFFFFE0000007FFFFF80000001FFFFE000000003FFF00000";
    
    // Create event handlers
    let generate_new_wallet_handler = generate_new_wallet(
        mnemonic.clone(),
        password.clone(),
        confirm_password.clone(),
        show_password_modal.clone(),
    );
    
    let confirm_password_and_show_mnemonic_handler = confirm_password_and_show_mnemonic(
        password.clone(),
        confirm_password.clone(),
        error_message.clone(),
        show_password_modal.clone(),
        show_modal.clone(),
        wallet_saved.clone(),
    );
    
    let show_import_wallet_modal_handler = show_import_wallet_modal(
        import_mnemonic.clone(),
        error_message.clone(),
        show_import_modal.clone(),
    );
    
    let confirm_import_mnemonic_handler = confirm_import_mnemonic(
        import_mnemonic.clone(),
        error_message.clone(),
        show_import_modal.clone(),
        import_password.clone(),
        import_confirm_password.clone(),
        show_import_password_modal.clone(),
    );
    
    let confirm_import_password_handler = confirm_import_password(
        import_mnemonic.clone(),
        import_password.clone(),
        import_confirm_password.clone(),
        error_message.clone(),
        is_importing.clone(),
        wallet_address.clone(),
        wallet_saved.clone(),
        show_import_password_modal.clone(),
        wallet_balance.clone(),
        is_loading_balance.clone(),
        session_active.clone(),
    );
    
    let show_decrypt_dialog_handler = show_decrypt_dialog(
        decryption_password.clone(),
        show_decrypt_modal.clone(),
        is_decrypting.clone(),
        error_message.clone(),
    );
    
    let decrypt_mnemonic_handler = decrypt_mnemonic(
        is_decrypting.clone(),
        decryption_password.clone(),
        error_message.clone(),
        mnemonic.clone(),
        show_modal.clone(),
        show_decrypt_modal.clone(),
        session_active.clone(),
        wallet_address.clone(),
    );
    
    let clear_wallet_handler = clear_wallet(
        wallet_address.clone(),
        wallet_saved.clone(),
        mnemonic.clone(),
        error_message.clone(),
        wallet_balance.clone(),
        session_active.clone(),
    );
    
    let refresh_balance_handler = refresh_balance(
        wallet_address.clone(),
        wallet_balance.clone(),
        is_loading_balance.clone(),
    );
    
    let handle_session_password_submit_handler = handle_session_password_submit(
        session_password_error.clone(),
        is_decrypting.clone(),
        show_session_password_modal.clone(),
        session_active.clone(),
        wallet_address.clone(),
        wallet_balance.clone(),
        is_loading_balance.clone(),
    );
    
    let handle_session_password_cancel_handler = handle_session_password_cancel(
        show_session_password_modal.clone(),
        wallet_saved.clone(),
    );
    
    let show_send_transaction_modal_handler = show_send_transaction_modal(
        recipient_address.clone(),
        send_amount.clone(),
        transaction_memo.clone(),
        error_message.clone(),
        show_send_modal.clone(),
        session_active.clone(),
    );
    
    let sign_and_send_transaction = create_sign_and_send_transaction(
        is_sending.clone(),
        error_message.clone(),
        prepared_transaction.clone(),
        show_transaction_password_modal.clone(),
        show_send_modal.clone(),
        session_active.clone(),
    );
    let sign_and_send_transaction = Rc::new(RefCell::new(sign_and_send_transaction));
    
    let prepare_transaction_handler = prepare_transaction(
        recipient_address.clone(),
        send_amount.clone(),
        transaction_memo.clone(),
        error_message.clone(),
        wallet_address.clone(),
        prepared_transaction.clone(),
        wallet_balance.clone(),
        transaction_password.clone(),
        show_send_modal.clone(),
        show_transaction_password_modal.clone(),
        sign_and_send_transaction.clone(),
        session_active.clone(),
    );
    
    let confirm_transaction_with_password_handler = confirm_transaction_with_password(
        transaction_password.clone(),
        error_message.clone(),
        sign_and_send_transaction.clone(),
    );
    
    let close_send_modal_handler = close_send_modal(show_send_modal.clone());
    let close_transaction_password_modal_handler = close_transaction_password_modal(show_transaction_password_modal.clone());
    
    // Mint handlers
    let show_mint_modal_handler = move |_: MouseEvent| {
        error_message.set("".to_string());
        show_mint_modal.set(true);
        log::info!("Mint button clicked, showing mint modal. Session active: {}", session_active.read());
    };
    
    let close_mint_modal_handler = close_mint_modal(show_mint_modal.clone());
    
    let process_mint_handler = process_mint(
        error_message.clone(),
        is_minting.clone(),
        show_mint_modal.clone(),
        session_active.clone(),
        memo_name.clone(),
        memo_description.clone()
    );
    
    let close_import_modal = move |_: MouseEvent| {
        show_import_modal.set(false);
    };
    
    let close_import_password_modal = move |_: MouseEvent| {
        show_import_password_modal.set(false);
    };
    
    let close_password_modal = move |_: MouseEvent| {
        show_password_modal.set(false);
    };
    
    let close_decrypt_modal = move |_: MouseEvent| {
        show_decrypt_modal.set(false);
        decryption_password.set(String::new());
    };
    
    let _close_modal = move |_: ()| {
        if !*wallet_saved.read() {
            log::warn!("Modal closed without saving wallet");
        }
        show_modal.set(false);
    };
    
    // Try to load existing wallet on component mount
    use_effect(move || {
        // Debug: Print localStorage content
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(local_storage)) = window.local_storage() {
                    if let Ok(Some(wallet_data)) = local_storage.get_item("wallet_data") {
                        log::info!("Raw wallet data in localStorage: {}", wallet_data);
                    } else {
                        log::info!("No wallet_data found in localStorage");
                    }
                }
            }
        }
        
        // Load wallet data
        match storage::load_wallet() {
            Ok(Some(wallet)) => {
                wallet_saved.set(true);
                
                // Check if we have an active session
                if session::is_session_active() {
                    // Session is active, get the wallet address
                    if let Some(address) = wallet::get_wallet_address() {
                        let address_clone = address.clone();
                        wallet_address.set(address);
                        log::info!("Loaded existing wallet with address: {}", address_clone);
                        
                        // Fetch wallet balance
                        fetch_balance(address_clone, wallet_balance.clone(), is_loading_balance.clone());
                    }
                } else {
                    // No active session, show password modal
                    log::info!("Wallet exists but no active session, showing password modal");
                    show_session_password_modal.set(true);
                }
            },
            Ok(None) => {
                log::info!("No existing wallet found");
            },
            Err(err) => {
                error_message.set(format!("Failed to load wallet: {}", err));
                log::error!("Failed to load wallet: {}", err);
            }
        }
    });
    
    rsx! {
        // Header with wallet address
        header {
            class: "app-header",
            div { class: "app-title", "memo" }
            
            // Show wallet address if available
            {
                let addr = wallet_address.read();
                if !addr.is_empty() {
                    log::info!("Displaying wallet address: {}", addr);
                    rsx! {
                        WalletAddressDisplay { address: addr.clone() }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
        }
        
        div {
            class: "container",
            
            // Show wallet creation button only if no wallet exists
            {
                let is_wallet_saved = *wallet_saved.read();
                log::info!("Rendering UI with wallet_saved: {}", is_wallet_saved);
                
                if !is_wallet_saved {
                    rsx! {
                        div { class: "wallet-creation",
                            button {
                                class: "new-wallet-btn",
                                onclick: generate_new_wallet_handler,
                                "New Wallet"
                            }
                            
                            button {
                                class: "import-wallet-btn",
                                onclick: show_import_wallet_modal_handler,
                                "Import Wallet"
                            }
                        }
                    }
                } else {
                    rsx! { 
                        div { class: "wallet-dashboard",
                            h1 { "Memo Inscription - X1 Testnet" }
                            
                            // Add session status indicator
                            {
                                if *session_active.read() {
                                    rsx! {
                                        div { class: "session-indicator active",
                                            "Wallet Session Active"
                                        }
                                    }
                                } else {
                                    rsx! {
                                        div { class: "session-indicator inactive",
                                            "Wallet Session Inactive"
                                        }
                                    }
                                }
                            }
                            
                            div { class: "wallet-balance",
                                div { class: "balance-header",
                                    h2 { "Balance" }
                                    button {
                                        class: "refresh-btn",
                                        onclick: refresh_balance_handler,
                                        disabled: *is_loading_balance.read(),
                                        i { class: "refresh-icon" }
                                    }
                                }
                                
                                {
                                    if *is_loading_balance.read() {
                                        rsx! {
                                            p { class: "balance-loading", "Loading..." }
                                        }
                                    } else if let Some(balance) = *wallet_balance.read() {
                                        rsx! {
                                            p { class: "balance-amount", "{balance:.9} {config::TOKEN_SYMBOL}" }
                                            p { class: "balance-usd", "($ 0.00 USD)" }
                                            p { class: "balance-network", "Network: {config::NETWORK_NAME}" }
                                        }
                                    } else {
                                        rsx! {
                                            p { class: "balance-amount", "0 {config::TOKEN_SYMBOL}" }
                                            p { class: "balance-usd", "($ 0.00 USD)" }
                                            p { class: "balance-network", "Network: {config::NETWORK_NAME}" }
                                        }
                                    }
                                }
                            }
                            
                            div { class: "wallet-actions",
                                button { class: "action-btn receive-btn", "Receive" }
                                button { 
                                    class: "action-btn send-btn", 
                                    onclick: show_send_transaction_modal_handler,
                                    "Send" 
                                }
                                button { 
                                    class: "action-btn mint-btn", 
                                    onclick: show_mint_modal_handler,
                                    "Mint" 
                                }
                            }
                            
                            // Add Memo Display with Pixel Canvas
                            div { class: "memo",
                                div { class: "memo-title", "Memo Inscription" }
                                PixelCanvas { hex_string: pixel_hex.to_string() }
                            }
                            
                            div { class: "transaction-history",
                                h2 { "Recent Transactions" }
                                p { class: "no-transactions", "No transactions yet" }
                            }
                            
                            // Add buttons for wallet management
                            div { class: "wallet-management",
                                button {
                                    class: "action-btn view-mnemonic-btn",
                                    onclick: show_decrypt_dialog_handler,
                                    "View Recovery Phrase"
                                }
                                
                                button {
                                    class: "action-btn clear-btn",
                                    onclick: clear_wallet_handler,
                                    "Clear Wallet (Debug)"
                                }
                            }
                        }
                    }
                }
            }
            
            // Show success message if wallet was saved
            {
                if *wallet_saved.read() {
                    rsx! {
                        div { class: "success-message",
                            "Wallet created and saved successfully!"
                        }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
            
            // Show error message if there was an error
            {
                let err = error_message.read();
                if !err.is_empty() {
                    rsx! {
                        div { class: "error-message",
                            "{err}"
                        }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
            
            // Password input modal
            {
                if *show_password_modal.read() {
                    rsx! {
                        div { class: "modal-overlay",
                            div { class: "modal password-modal",
                                h2 { "Set Wallet Password" }
                                p { "This password will be used to encrypt your recovery phrase." }
                                
                                div { class: "form-group",
                                    label { "Password:" }
                                    input {
                                        r#type: "password",
                                        value: "{password}",
                                        oninput: move |evt| password.set(evt.value().clone()),
                                        placeholder: "Enter password"
                                    }
                                }
                                
                                div { class: "form-group",
                                    label { "Confirm Password:" }
                                    input {
                                        r#type: "password",
                                        value: "{confirm_password}",
                                        oninput: move |evt| confirm_password.set(evt.value().clone()),
                                        placeholder: "Confirm password"
                                    }
                                }
                                
                                div { class: "modal-actions",
                                    button {
                                        class: "cancel-btn",
                                        onclick: close_password_modal,
                                        "Cancel"
                                    }
                                    button {
                                        class: "confirm-btn",
                                        onclick: confirm_password_and_show_mnemonic_handler,
                                        "Confirm"
                                    }
                                }
                            }
                        }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
            
            // Mnemonic decryption modal
            {
                if *show_decrypt_modal.read() {
                    rsx! {
                        div { class: "modal-overlay",
                            div { class: "modal decrypt-modal",
                                h2 { "Enter Wallet Password" }
                                p { "Enter your password to view your recovery phrase." }
                                
                                div { class: "form-group",
                                    label { "Password:" }
                                    input {
                                        r#type: "password",
                                        value: "{decryption_password}",
                                        oninput: move |evt| decryption_password.set(evt.value().clone()),
                                        placeholder: "Enter password"
                                    }
                                }
                                
                                div { class: "modal-actions",
                                    button {
                                        class: "cancel-btn",
                                        onclick: close_decrypt_modal,
                                        "Cancel"
                                    }
                                    button {
                                        class: "confirm-btn",
                                        onclick: decrypt_mnemonic_handler,
                                        disabled: *is_decrypting.read(),
                                        if *is_decrypting.read() {
                                            "Decrypting..."
                                        } else {
                                            "View Phrase"
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
            
            // Import wallet modal
            {
                if *show_import_modal.read() {
                    rsx! {
                        div { class: "modal-overlay",
                            div { class: "modal import-modal",
                                h2 { "Import Wallet" }
                                p { "Enter your recovery phrase (12 words separated by spaces)" }
                                
                                div { class: "form-group",
                                    textarea {
                                        class: "mnemonic-input",
                                        value: "{import_mnemonic}",
                                        oninput: move |evt| import_mnemonic.set(evt.value().clone()),
                                        placeholder: "Enter your 12-word recovery phrase"
                                    }
                                }
                                
                                div { class: "modal-actions",
                                    button {
                                        class: "cancel-btn",
                                        onclick: close_import_modal,
                                        "Cancel"
                                    }
                                    button {
                                        class: "confirm-btn",
                                        onclick: confirm_import_mnemonic_handler,
                                        "Continue"
                                    }
                                }
                            }
                        }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
            
            // Import wallet password modal
            {
                if *show_import_password_modal.read() {
                    rsx! {
                        div { class: "modal-overlay",
                            div { class: "modal password-modal",
                                h2 { "Set Wallet Password" }
                                p { "This password will be used to encrypt your recovery phrase." }
                                
                                div { class: "form-group",
                                    label { "Password:" }
                                    input {
                                        r#type: "password",
                                        value: "{import_password}",
                                        oninput: move |evt| import_password.set(evt.value().clone()),
                                        placeholder: "Enter password"
                                    }
                                }
                                
                                div { class: "form-group",
                                    label { "Confirm Password:" }
                                    input {
                                        r#type: "password",
                                        value: "{import_confirm_password}",
                                        oninput: move |evt| import_confirm_password.set(evt.value().clone()),
                                        placeholder: "Confirm password"
                                    }
                                }
                                
                                div { class: "modal-actions",
                                    button {
                                        class: "cancel-btn",
                                        onclick: close_import_password_modal,
                                        "Cancel"
                                    }
                                    button {
                                        class: "confirm-btn",
                                        onclick: confirm_import_password_handler,
                                        disabled: *is_importing.read(),
                                        if *is_importing.read() {
                                            "Importing..."
                                        } else {
                                            "Import Wallet"
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
            
            // Add send transaction modal
            {
                if *show_send_modal.read() {
                    rsx! {
                        div { class: "modal-overlay",
                            div { class: "modal send-modal",
                                h2 { "Send {config::TOKEN_SYMBOL}" }
                                
                                div { class: "form-group",
                                    label { "Recipient Address:" }
                                    input {
                                        r#type: "text",
                                        value: "{recipient_address}",
                                        oninput: move |evt| recipient_address.set(evt.value().clone()),
                                        placeholder: "Enter recipient address"
                                    }
                                }
                                
                                div { class: "form-group",
                                    label { "Amount:" }
                                    input {
                                        r#type: "text",
                                        value: "{send_amount}",
                                        oninput: move |evt| send_amount.set(evt.value().clone()),
                                        placeholder: "Enter amount to send"
                                    }
                                }
                                
                                div { class: "form-group",
                                    label { "Memo (optional):" }
                                    input {
                                        r#type: "text",
                                        value: "{transaction_memo}",
                                        oninput: move |evt| transaction_memo.set(evt.value().clone()),
                                        placeholder: "Enter memo (optional)"
                                    }
                                }
                                
                                // Show session status
                                {
                                    if *session_active.read() {
                                        rsx! {
                                            div { class: "session-status active",
                                                "Wallet session active - No password needed"
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            div { class: "session-status inactive",
                                                "Wallet session inactive - Password will be required"
                                            }
                                        }
                                    }
                                }
                                
                                div { class: "modal-actions",
                                    button {
                                        class: "cancel-btn",
                                        onclick: close_send_modal_handler,
                                        "Cancel"
                                    }
                                    button {
                                        class: "confirm-btn",
                                        onclick: prepare_transaction_handler,
                                        "Send"
                                    }
                                }
                            }
                        }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
            
            // Add transaction password modal
            {
                if *show_transaction_password_modal.read() {
                    rsx! {
                        div { class: "modal-overlay",
                            div { class: "modal password-modal",
                                h2 { "Enter Wallet Password" }
                                p { "Enter your password to sign the transaction." }
                                
                                div { class: "form-group",
                                    label { "Password:" }
                                    input {
                                        r#type: "password",
                                        value: "{transaction_password}",
                                        oninput: move |evt| transaction_password.set(evt.value().clone()),
                                        placeholder: "Enter password"
                                    }
                                }
                                
                                div { class: "modal-actions",
                                    button {
                                        class: "cancel-btn",
                                        onclick: close_transaction_password_modal_handler,
                                        "Cancel"
                                    }
                                    button {
                                        class: "confirm-btn",
                                        onclick: confirm_transaction_with_password_handler,
                                        disabled: *is_sending.read(),
                                        if *is_sending.read() {
                                            "Signing..."
                                        } else {
                                            "Sign & Send"
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
            
            // Add Mint modal
            {
                if *show_mint_modal.read() {
                    rsx! {
                        div { class: "modal-overlay",
                            div { class: "modal mint-modal",
                                h2 { "Mint Memo Inscription" }
                                p { "Create your own Memo Inscription on the X1 Testnet" }
                                
                                div { class: "form-group",
                                    label { "Memo Name:" }
                                    input {
                                        r#type: "text",
                                        value: "{memo_name}",
                                        oninput: move |evt| memo_name.set(evt.value().clone()),
                                        placeholder: "Enter memo inscription name"
                                    }
                                }
                                
                                div { class: "form-group",
                                    label { "Description:" }
                                    textarea {
                                        class: "memo-description",
                                        value: "{memo_description}",
                                        oninput: move |evt| memo_description.set(evt.value().clone()),
                                        placeholder: "Enter memo inscription description"
                                    }
                                }
                                
                                // Add Pixel Canvas Preview
                                div { class: "pixel-preview",
                                    h3 { "Memo Inscription Preview:" }
                                    PixelCanvas { hex_string: pixel_hex.to_string() }
                                }
                                
                                // Show session status
                                {
                                    if *session_active.read() {
                                        rsx! {
                                            div { class: "session-status active",
                                                "Wallet session active - Ready to mint"
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            div { class: "session-status inactive",
                                                "Wallet session inactive - Please unlock your wallet first"
                                            }
                                        }
                                    }
                                }
                                
                                div { class: "modal-actions",
                                    button {
                                        class: "cancel-btn",
                                        onclick: close_mint_modal_handler,
                                        "Cancel"
                                    }
                                    button {
                                        class: "confirm-btn",
                                        onclick: process_mint_handler,
                                        disabled: *is_minting.read(),
                                        if *is_minting.read() {
                                            "Minting..."
                                        } else {
                                            "Mint Memo Inscription"
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
            
            // Session password modal
            PasswordModal {
                visible: *show_session_password_modal.read(),
                on_submit: handle_session_password_submit_handler,
                on_cancel: handle_session_password_cancel_handler,
                error_message: Some(session_password_error.read().clone()),
                is_loading: Some(*is_decrypting.read())
            }
            
            // Render the mnemonic modal
            MnemonicModal {
                mnemonic: mnemonic.read().clone(),
                visible: *show_modal.read(),
                on_close: _close_modal
            }
        }
    }
} 