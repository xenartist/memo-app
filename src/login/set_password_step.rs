use leptos::*;
use crate::CreateWalletStep;
use crate::core::wallet::{
    generate_seed_from_mnemonic,
    store_encrypted_seed,
    derive_keypair_from_seed,
    get_default_derivation_path
};
use crate::core::encrypt;
use crate::core::NetworkType;
use hex;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn SetPasswordStep(
    mnemonic: ReadSignal<String>,
    password: ReadSignal<String>,
    set_password: WriteSignal<String>,
    set_current_step: WriteSignal<CreateWalletStep>,
    set_wallet_address: WriteSignal<String>,
    set_encrypted_seed: WriteSignal<String>,
    selected_network: RwSignal<NetworkType>,
) -> impl IntoView {
    let (show_passphrase, set_show_passphrase) = create_signal(false);
    let (passphrase, set_passphrase) = create_signal(String::new());
    let (passphrase_confirm, set_passphrase_confirm) = create_signal(String::new());
    let (password_input, set_password_input) = create_signal(String::new());
    let (password_confirm, set_password_confirm) = create_signal(String::new());
    let (error_message, set_error_message) = create_signal(String::new());
    
    // add loading status
    let (is_encrypting, set_is_encrypting) = create_signal(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        
        if password_input.get() != password_confirm.get() {
            set_error_message.set("Passwords do not match".to_string());
            return;
        }
        
        if show_passphrase.get() {
            if passphrase.get() != passphrase_confirm.get() {
                set_error_message.set("Passphrases do not match".to_string());
                return;
            }
        }

        // set loading status
        set_is_encrypting.set(true);
        set_error_message.set(String::new());
    
        let mnemonic_owned = mnemonic.get().to_string();
        let password_owned = password_input.get().to_string();
        let passphrase_owned = if show_passphrase.get() {
            Some(passphrase.get().to_string())
        } else {
            None
        };
    
        spawn_local(async move {
            // give UI some time to update status
            TimeoutFuture::new(100).await;
            
            let passphrase_ref = passphrase_owned.as_deref();
            
            match generate_seed_from_mnemonic(&mnemonic_owned, passphrase_ref) {
                Ok(seed) => {
                    match derive_keypair_from_seed(&seed, get_default_derivation_path()) {
                        Ok((_, address)) => {
                            set_wallet_address.set(address);
                            
                            // use async encrypt function
                            match encrypt::encrypt_async(&hex::encode(seed), &password_owned).await {
                                Ok(encrypted) => {
                                    set_encrypted_seed.set(encrypted.clone());
                                    
                                    match store_encrypted_seed(&seed, &password_owned).await {
                                        Ok(()) => {
                                            set_password.set(password_owned);
                                            set_current_step.set(CreateWalletStep::Complete);
                                        }
                                        Err(_) => {
                                            set_error_message.set("Failed to store encrypted seed".to_string());
                                            set_is_encrypting.set(false);
                                        }
                                    }
                                }
                                Err(e) => {
                                    set_error_message.set(format!("Failed to encrypt seed: {}", e));
                                    set_is_encrypting.set(false);
                                }
                            }
                        }
                        Err(_) => {
                            set_error_message.set("Failed to derive keypair".to_string());
                            set_is_encrypting.set(false);
                        }
                    }
                }
                Err(_) => {
                    set_error_message.set("Failed to generate seed from mnemonic".to_string());
                    set_is_encrypting.set(false);
                }
            }
        });
    };

    view! {
        <div class="login-container">
            <div class="header-with-back">
                <button 
                    class="back-btn"
                    on:click=move |_| {
                        // return different steps based on whether it's importing or creating a wallet
                        if mnemonic.get().is_empty() {
                            set_current_step.set(CreateWalletStep::ImportMnemonic)
                        } else {
                            set_current_step.set(CreateWalletStep::VerifyMnemonic(mnemonic.get()))
                        }
                    }
                    // disable back button when encrypting
                    prop:disabled=move || is_encrypting.get()
                >
                    "← Back"
                </button>
                <h2>"Set Password"</h2>
            </div>
            
            // Display selected network (read-only)
            <div class="info-message" style="margin: 1rem auto; max-width: 500px;">
                <i class="fas fa-network-wired"></i>
                <span>
                    "Network: "
                    {move || match selected_network.get() {
                        NetworkType::Testnet => "Testnet",
                        NetworkType::ProdStaging => "Prod Staging",
                        NetworkType::Mainnet => "Mainnet",
                    }}
                </span>
            </div>
            
            <div class="advanced-options">
                <label class="checkbox-label">
                    <input
                        type="checkbox"
                        on:change=move |ev| {
                            let checked = event_target_checked(&ev);
                            set_show_passphrase.set(checked);
                        }
                        // disable checkbox when encrypting
                        prop:disabled=move || is_encrypting.get()
                    />
                    <i class="fas fa-lock"></i>
                    " Set BIP39 passphrase"
                </label>
                <p class="warning-text">
                    <i class="fas fa-exclamation-triangle"></i>
                    " Only enable this if you understand BIP39 passphrase. This adds an additional layer of security but requires careful management."
                </p>
            </div>

            <form on:submit=on_submit>
                // BIP39 passphrase (conditional display)
                {move || show_passphrase.get().then(|| view! {
                    <div class="passphrase-section">
                        <h3 class="section-title">
                            <i class="fas fa-key"></i>
                            " BIP39 Passphrase"
                        </h3>
                        <div class="input-group">
                            <input
                                type="password"
                                placeholder="Enter BIP39 passphrase"
                                on:input=move |ev| {
                                    set_passphrase.set(event_target_value(&ev));
                                }
                                prop:disabled=move || is_encrypting.get()
                                required
                            />
                        </div>
                        <div class="input-group">
                            <input
                                type="password"
                                placeholder="Confirm BIP39 passphrase"
                                on:input=move |ev| {
                                    set_passphrase_confirm.set(event_target_value(&ev));
                                }
                                prop:disabled=move || is_encrypting.get()
                                required
                            />
                        </div>
                    </div>
                })}

                // normal password input box
                <div class="password-section">
                    <h3 class="section-title">
                        <i class="fas fa-shield-alt"></i>
                        " Wallet Password"
                    </h3>
                    <div class="input-group">
                        <input
                            type="password"
                            placeholder="Enter wallet password"
                            on:input=move |ev| {
                                set_password_input.set(event_target_value(&ev));
                            }
                            prop:disabled=move || is_encrypting.get()
                            required
                        />
                    </div>
                    <div class="input-group">
                        <input
                            type="password"
                            placeholder="Confirm wallet password"
                            on:input=move |ev| {
                                set_password_confirm.set(event_target_value(&ev));
                            }
                            prop:disabled=move || is_encrypting.get()
                            required
                        />
                    </div>
                </div>

                // display encrypting status
                {move || {
                    if is_encrypting.get() {
                        view! {
                            <div class="encrypting-status">
                                <i class="fas fa-spinner fa-spin"></i>
                                <span>"Encrypting wallet data..."</span>
                            </div>
                        }
                    } else {
                        view! { <div></div> }
                    }
                }}

                <div class="error-message">
                    {move || if !error_message.get().is_empty() {
                        view! {
                            <i class="fas fa-exclamation-circle"></i>
                            <span>{error_message.get()}</span>
                        }.into_view()
                    } else {
                        view! { <></> }.into_view()
                    }}
                </div>

                <button 
                    type="submit" 
                    class="wallet-btn"
                    prop:disabled=move || is_encrypting.get()
                >
                    {move || if is_encrypting.get() { 
                        view! {
                            <i class="fas fa-spinner fa-spin"></i>
                            " Encrypting..."
                        }.into_view()
                    } else { 
                        view! {
                            <i class="fas fa-arrow-right"></i>
                            " Continue"
                        }.into_view()
                    }}
                </button>
            </form>
        </div>
    }
}