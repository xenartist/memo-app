use leptos::*;
use crate::CreateWalletStep;
use crate::core::session::Session;
use crate::core::{NetworkType, initialize_network};
use crate::core::backpack::BackpackWallet;

#[component]
pub fn BackpackConnectStep(
    set_current_step: WriteSignal<CreateWalletStep>,
    session: RwSignal<Session>,
    set_show_main_page: WriteSignal<bool>,
    selected_network: RwSignal<NetworkType>,
) -> impl IntoView {
    let (error_message, set_error_message) = create_signal(String::new());
    let (is_connecting, set_is_connecting) = create_signal(false);
    let (connection_status, set_connection_status) = create_signal(String::new());

    // Check if Backpack is installed
    let backpack_installed = BackpackWallet::is_installed();

    // Handle Backpack connection
    let handle_connect = move |_| {
        set_is_connecting.set(true);
        set_error_message.set(String::new());
        set_connection_status.set("Connecting to Backpack wallet...".to_string());

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

            // Connect to Backpack and initialize session
            match current_session.initialize_with_backpack().await {
                Ok(pubkey) => {
                    log::info!("Successfully connected to Backpack: {}", pubkey);
                    set_connection_status.set(format!("Connected: {}", pubkey));
                    
                    // Update session
                    session.set(current_session);
                    
                    // Navigate to main page
                    set_is_connecting.set(false);
                    set_show_main_page.set(true);
                }
                Err(e) => {
                    log::error!("Failed to connect to Backpack: {}", e);
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
            
            <h1 class="app-title">"Connect Backpack Wallet"</h1>
            
            {move || if backpack_installed {
                view! {
                    <div class="backpack-connect-container">
                        <div class="backpack-info">
                            <div class="backpack-icon-large">"🎒"</div>
                            <h2>"Backpack Wallet Detected"</h2>
                            <p class="info-text">
                                "Click the button below to connect your Backpack wallet. "
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
                                class="wallet-btn backpack-connect-btn"
                                on:click=handle_connect
                                disabled=move || is_connecting.get()
                            >
                                {move || if is_connecting.get() {
                                    "Connecting..."
                                } else {
                                    "Connect Backpack Wallet"
                                }}
                            </button>
                        </div>
                    </div>
                }.into_view()
            } else {
                view! {
                    <div class="backpack-connect-container">
                        <div class="backpack-not-installed">
                            <div class="warning-icon">"⚠️"</div>
                            <h2>"Backpack Wallet Not Found"</h2>
                            <p class="info-text">
                                "Backpack wallet is not installed in your browser. "
                                "Please install it first to continue."
                            </p>
                            
                            <div class="button-group">
                                <a
                                    href="https://www.backpack.app"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    class="wallet-btn install-backpack-btn"
                                >
                                    "Install Backpack Wallet"
                                </a>
                            </div>
                        </div>
                    </div>
                }.into_view()
            }}
        </div>
    }
}

