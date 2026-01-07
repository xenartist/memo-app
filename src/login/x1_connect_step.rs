use leptos::*;
use crate::CreateWalletStep;
use crate::core::session::Session;
use crate::core::{NetworkType, initialize_network};
use crate::core::x1::X1Wallet;

#[component]
pub fn X1ConnectStep(
    set_current_step: WriteSignal<CreateWalletStep>,
    session: RwSignal<Session>,
    set_show_main_page: WriteSignal<bool>,
    selected_network: RwSignal<NetworkType>,
) -> impl IntoView {
    let (error_message, set_error_message) = create_signal(String::new());
    let (is_connecting, set_is_connecting) = create_signal(false);
    let (connection_status, set_connection_status) = create_signal(String::new());

    // Check if X1 is installed
    let x1_installed = X1Wallet::is_installed();

    // Handle X1 connection
    let handle_connect = move |_| {
        set_is_connecting.set(true);
        set_error_message.set(String::new());
        set_connection_status.set("Connecting to X1 wallet...".to_string());

        let session = session.clone();
        let selected_network = selected_network.clone();

        spawn_local(async move {
            // Initialize network first
            let network = selected_network.get_untracked();
            if !initialize_network(network) {
                set_error_message.set("Failed to initialize network".to_string());
                set_is_connecting.set(false);
                set_connection_status.set(String::new());
                return;
            }

            // Get current session
            let mut current_session = session.get_untracked();
            
            // Set network in session
            current_session.set_network(network);

            // Connect to X1 and initialize session
            match current_session.initialize_with_x1().await {
                Ok(pubkey) => {
                    log::info!("Successfully connected to X1: {}", pubkey);
                    set_connection_status.set(format!("Connected: {}", pubkey));
                    
                    // Update session
                    session.set(current_session);
                    
                    // Navigate to main page
                    set_is_connecting.set(false);
                    set_show_main_page.set(true);
                }
                Err(e) => {
                    log::error!("Failed to connect to X1: {}", e);
                    set_error_message.set(format!("Failed to connect: {}", e));
                    set_is_connecting.set(false);
                    set_connection_status.set(String::new());
                }
            }
        });
    };

    view! {
        <div class="login-container">
            // Back button in top-left corner
            <button
                class="back-btn-corner"
                on:click=move |_| set_current_step.set(CreateWalletStep::Initial)
                disabled=move || is_connecting.get()
            >
                "← Back"
            </button>
            
            <h1 class="app-title">"Connect X1 Wallet"</h1>
            
            {move || if x1_installed {
                view! {
                    <div class="x1-connect-container">
                        <div class="x1-info">
                            <img src="https://x1logos.s3.us-east-1.amazonaws.com/128+-+wallet.png" alt="X1" class="x1-icon-large-img" />
                            <h2>"X1 Wallet Detected"</h2>
                            <p class="info-text">
                                "Click the button below to connect your X1 wallet. "
                                "You will be prompted to approve the connection."
                            </p>
                        </div>

                        {move || if !connection_status.get().is_empty() {
                            view! {
                                <div class="connection-status">
                                    <div class="spinner"></div>
                                    <p>{connection_status.get()}</p>
                                </div>
                            }.into_view()
                        } else {
                            view! { <div></div> }.into_view()
                        }}

                        {move || if !error_message.get().is_empty() {
                            view! {
                                <div class="error-message">
                                    <i class="fas fa-exclamation-circle"></i>
                                    " " {error_message.get()}
                                </div>
                            }.into_view()
                        } else {
                            view! { <div></div> }.into_view()
                        }}

                        <div class="button-group">
                            <button
                                class="wallet-btn x1-connect-btn"
                                on:click=handle_connect
                                disabled=move || is_connecting.get()
                            >
                                {move || if is_connecting.get() {
                                    "Connecting..."
                                } else {
                                    "Connect X1 Wallet"
                                }}
                            </button>
                        </div>
                    </div>
                }.into_view()
            } else {
                view! {
                    <div class="x1-connect-container">
                        <div class="x1-not-installed">
                            <div class="warning-icon">"⚠️"</div>
                            <h2>"X1 Wallet Not Found"</h2>
                            <p class="info-text">
                                "X1 wallet is not installed in your browser. "
                                "Please install it first to continue."
                            </p>
                            
                            <div class="button-group">
                                <a
                                    href="https://x1.xyz"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    class="wallet-btn install-x1-btn"
                                >
                                    "Install X1 Wallet"
                                </a>
                            </div>
                        </div>
                    </div>
                }.into_view()
            }}
        </div>
    }
}

