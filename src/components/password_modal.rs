use dioxus::prelude::*;

/// Password input modal for unlocking wallet
#[component]
pub fn PasswordModal(
    visible: bool,
    on_submit: EventHandler<String>,
    on_cancel: EventHandler<()>,
    error_message: Option<String>,
    is_loading: Option<bool>,
) -> Element {
    let mut password = use_signal(|| String::new());
    let mut internal_error = use_signal(|| String::new());
    
    // Use provided error message if available
    let display_error = error_message.unwrap_or_else(|| internal_error.read().clone());
    
    // Use provided loading state if available
    let is_loading_state = is_loading.unwrap_or(false);
    
    // Handle password submission
    let handle_submit = move |evt: FormEvent| {
        evt.prevent_default();
        
        if password.read().is_empty() {
            internal_error.set("Password cannot be empty".to_string());
            return;
        }
        
        on_submit.call(password.read().clone());
    };
    
    // Don't render if not visible
    if !visible {
        return rsx!{ Fragment {} };
    }
    
    rsx! {
        div {
            class: "modal-overlay",
            div {
                class: "modal-content password-modal",
                // Prevent clicks inside the modal from closing it
                onclick: move |evt| evt.stop_propagation(),
                h2 { "Unlock Your Wallet" }
                p { "Enter your password to access your wallet." }
                
                // Error message if any
                {
                    if !display_error.is_empty() {
                        rsx! {
                            div { class: "error-message", "{display_error}" }
                        }
                    } else {
                        rsx! { Fragment {} }
                    }
                }
                
                form {
                    onsubmit: handle_submit,
                    div { class: "form-group",
                        label { r#for: "password", "Password:" }
                        input {
                            id: "password",
                            r#type: "password",
                            value: "{password}",
                            oninput: move |evt| password.set(evt.value().clone()),
                            placeholder: "Enter your wallet password",
                            autocomplete: "current-password",
                            disabled: is_loading_state
                        }
                    }
                    
                    div { class: "modal-actions",
                        button {
                            r#type: "submit",
                            class: "primary-button",
                            disabled: is_loading_state,
                            if is_loading_state {
                                "Unlocking..."
                            } else {
                                "Unlock"
                            }
                        }
                        button {
                            r#type: "button",
                            class: "secondary-button",
                            onclick: move |_| on_cancel.call(()),
                            disabled: is_loading_state,
                            "Cancel"
                        }
                    }
                }
            }
        }
    }
} 