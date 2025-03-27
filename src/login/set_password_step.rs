use leptos::*;
use crate::CreateWalletStep;
use crate::wallet::{generate_keypair_from_mnemonic, store_encrypted_keypair};

#[component]
pub fn SetPasswordStep(
    mnemonic: ReadSignal<String>,
    password: ReadSignal<String>,
    set_password: WriteSignal<String>,
    set_current_step: WriteSignal<CreateWalletStep>,
    set_wallet_address: WriteSignal<String>,
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
    
        // Clone values
        let mnemonic_owned = mnemonic.get().to_string();
        let password_owned = password_input.get().to_string();
        let passphrase_owned = if show_passphrase.get() {
            Some(passphrase.get().to_string())
        } else {
            None
        };
    
        spawn_local(async move {
            let passphrase_ref = passphrase_owned.as_deref();
            
            match generate_keypair_from_mnemonic(&mnemonic_owned, passphrase_ref) {
                Ok((keypair, address)) => {
                    // save address for display
                    set_wallet_address.set(address);
                    
                    // encrypt and store the main private key
                    match store_encrypted_keypair(&keypair, &password_owned).await {
                        Ok(_) => {
                            set_password.set(password_owned);
                            set_current_step.set(CreateWalletStep::Complete);
                        }
                        Err(_) => {
                            set_error_message.set("Failed to store encrypted keypair".to_string());
                        }
                    }
                }
                Err(_) => {
                    set_error_message.set("Failed to generate keypair from mnemonic".to_string());
                }
            }
        });
    };

    view! {
        <div class="login-container">
            <h2>"Set Your Password"</h2>
            
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