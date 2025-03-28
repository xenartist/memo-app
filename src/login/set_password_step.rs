use leptos::*;
use crate::CreateWalletStep;
use crate::core::wallet::{
    generate_seed_from_mnemonic,
    store_encrypted_seed,
    derive_keypair_from_seed,
    get_default_derivation_path
};
use crate::core::encrypt;
use hex;

#[component]
pub fn SetPasswordStep(
    mnemonic: ReadSignal<String>,
    password: ReadSignal<String>,
    set_password: WriteSignal<String>,
    set_current_step: WriteSignal<CreateWalletStep>,
    set_wallet_address: WriteSignal<String>,
    set_encrypted_seed: WriteSignal<String>,
) -> impl IntoView {
    let (show_passphrase, set_show_passphrase) = create_signal(false);
    let (passphrase, set_passphrase) = create_signal(String::new());
    let (passphrase_confirm, set_passphrase_confirm) = create_signal(String::new());
    let (password_input, set_password_input) = create_signal(String::new());
    let (password_confirm, set_password_confirm) = create_signal(String::new());
    let (error_message, set_error_message) = create_signal(String::new());

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
    
        let mnemonic_owned = mnemonic.get().to_string();
        let password_owned = password_input.get().to_string();
        let passphrase_owned = if show_passphrase.get() {
            Some(passphrase.get().to_string())
        } else {
            None
        };
    
        spawn_local(async move {
            let passphrase_ref = passphrase_owned.as_deref();
            
            match generate_seed_from_mnemonic(&mnemonic_owned, passphrase_ref) {
                Ok(seed) => {
                    match derive_keypair_from_seed(&seed, get_default_derivation_path()) {
                        Ok((_, address)) => {
                            set_wallet_address.set(address);
                            
                            if let Ok(encrypted) = encrypt::encrypt(&hex::encode(seed), &password_owned) {
                                set_encrypted_seed.set(encrypted.clone());
                                
                                if let Ok(()) = store_encrypted_seed(&seed, &password_owned).await {
                                    set_password.set(password_owned);
                                    set_current_step.set(CreateWalletStep::Complete);
                                } else {
                                    set_error_message.set("Failed to store encrypted seed".to_string());
                                }
                            } else {
                                set_error_message.set("Failed to encrypt seed".to_string());
                            }
                        }
                        Err(_) => {
                            set_error_message.set("Failed to derive keypair".to_string());
                        }
                    }
                }
                Err(_) => {
                    set_error_message.set("Failed to generate seed from mnemonic".to_string());
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
                >
                    "← Back"
                </button>
                <h2>"Set Password"</h2>
            </div>
            
            <div class="advanced-options">
                <label class="checkbox-label">
                    <input
                        type="checkbox"
                        on:change=move |ev| {
                            let checked = event_target_checked(&ev);
                            set_show_passphrase.set(checked);
                        }
                    />
                    "Set BIP39 passphrase"
                </label>
                <p class="warning-text">
                    "Only enable this if you understand BIP39 passphrase. This adds an additional layer of security but requires careful management."
                </p>
            </div>

            <form on:submit=on_submit>
                // BIP39 passphrase (conditional display)
                {move || show_passphrase.get().then(|| view! {
                    <div class="passphrase-section">
                        <h3 class="section-title">"BIP39 Passphrase"</h3>
                        <div class="input-group">
                            <input
                                type="password"
                                placeholder="Enter BIP39 passphrase"
                                on:input=move |ev| {
                                    set_passphrase.set(event_target_value(&ev));
                                }
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
                                required
                            />
                        </div>
                    </div>
                })}

                // normal password input box
                <div class="password-section">
                    <h3 class="section-title">"Wallet Password"</h3>
                    <div class="input-group">
                        <input
                            type="password"
                            placeholder="Enter wallet password"
                            on:input=move |ev| {
                                set_password_input.set(event_target_value(&ev));
                            }
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
                            required
                        />
                    </div>
                </div>

                <div class="error-message">
                    {move || error_message.get()}
                </div>

                <button type="submit" class="wallet-btn">
                    "Continue"
                </button>
            </form>
        </div>
    }
}