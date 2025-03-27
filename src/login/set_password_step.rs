use leptos::*;
use crate::CreateWalletStep;

#[component]
pub fn SetPasswordStep(
    mnemonic: ReadSignal<String>,
    password: ReadSignal<String>,
    set_password: WriteSignal<String>,
    set_current_step: WriteSignal<CreateWalletStep>,
) -> impl IntoView {
    let (show_passphrase, set_show_passphrase) = create_signal(false);
    let (passphrase, set_passphrase) = create_signal(String::new());
    let (passphrase_confirm, set_passphrase_confirm) = create_signal(String::new());
    let (password_input, set_password_input) = create_signal(String::new());
    let (password_confirm, set_password_confirm) = create_signal(String::new());
    let (error_message, set_error_message) = create_signal(String::new());

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        
        // verify password
        if password_input.get() != password_confirm.get() {
            set_error_message.set("Passwords do not match".to_string());
            return;
        }
        
        // if passphrase is enabled, verify passphrase
        if show_passphrase.get() {
            if passphrase.get() != passphrase_confirm.get() {
                set_error_message.set("Passphrases do not match".to_string());
                return;
            }
        }
        
        // set password and go to next step
        set_password.set(password_input.get());
        set_current_step.set(CreateWalletStep::Complete);
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
                // BIP39 passphrase input box (conditional display)
                {move || show_passphrase.get().then(|| view! {
                    <div class="passphrase-section">
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