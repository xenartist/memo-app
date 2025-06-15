use leptos::*;
use crate::core::session::Session;
use crate::core::storage_mint::get_mint_storage;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use crate::pages::mint_form::MintForm;
use std::rc::Rc;

#[component]
pub fn MintPage(
    session: RwSignal<Session>
) -> impl IntoView {
    // add storage status display
    let (storage_status, set_storage_status) = create_signal(String::new());
    
    // add signal to control mint form visibility
    let (show_mint_form, set_show_mint_form) = create_signal(false);

    // get storage status on initialization
    create_effect(move |_| {
        spawn_local(async move {
            log::info!("=== Storage Initialization Start ===");
            
            // add small delay to ensure DOM is ready
            TimeoutFuture::new(100).await;
            
            // first try to get sync status
            match get_mint_storage().get_storage_status() {
                Ok(basic_status) => {
                    log::info!("Basic storage status: {}", basic_status);
                    set_storage_status.set(basic_status);
                    
                    // then try to get detailed status
                    match get_mint_storage().get_detailed_storage_status().await {
                        Ok(detailed_status) => {
                            log::info!("Detailed storage status: {}", detailed_status);
                            set_storage_status.set(detailed_status);
                        },
                        Err(e) => {
                            log::error!("Failed to get detailed storage status: {}", e);
                            // keep basic status display
                        }
                    }
                },
                Err(e) => {
                    log::error!("Failed to get basic storage status: {}", e);
                    set_storage_status.set(format!("Storage Error: {}", e));
                }
            }
            
            log::info!("=== Storage Initialization End ===");
        });
    });

    // Optional callbacks for mint events
    let on_mint_success = Rc::new(move |signature: String, tokens_minted: u64, total_minted: u64| {
        log::info!("Mint successful on page level: {} tokens minted, total: {}", tokens_minted, total_minted);
        
        // Update storage status after successful mint
        spawn_local(async move {
            if let Ok(status) = get_mint_storage().get_detailed_storage_status().await {
                set_storage_status.set(status);
            }
        });
    });

    let on_mint_error = Rc::new(move |error: String| {
        log::error!("Mint error on page level: {}", error);
    });

    view! {
        <div class="mint-page">
            <div class="mint-page-header">
                <h2>"Mint"</h2>
                
                // display storage status information
                <div class="storage-status">
                    <span class="storage-info">
                        {move || {
                            let status = storage_status.get();
                            if status.is_empty() {
                                "🔄 Loading storage info...".to_string()
                            } else {
                                status
                            }
                        }}
                    </span>
                </div>
                
                // Action buttons
                <div class="mint-actions">
                    <button 
                        class="open-mint-form-btn"
                        on:click=move |_| set_show_mint_form.set(true)
                        disabled=move || !session.get().has_user_profile()
                    >
                        "🚀 Engrave & Mint"
                    </button>
                    
                    // Show warning when no profile
                    <Show when=move || !session.get().has_user_profile()>
                        <div class="no-profile-warning">
                            <p>"⚠️ Please create your mint profile in the Profile page before you can start minting."</p>
                        </div>
                    </Show>
                </div>
            </div>
            
            // Main content area (where you can add card lists, etc.)
            <div class="mint-content">
                <div class="welcome-section">
                    <h3>"Ready to Engrave Your Memories?"</h3>
                    <p>"Engrave your thoughts, ideas, and art into permanent memories on the blockchain."</p>
                    <p>"And mint random amount of MEMO tokens at the same time."</p>
                    
                    // Here you can add card lists or other content
                    <div class="content-placeholder">
                        <p>"🎨 Your minted memories will appear here..."</p>
                        <p>"📝 Recent transactions and history..."</p>
                        <p>"🖼️ Gallery of your pixel art creations..."</p>
                    </div>
                </div>
            </div>
            
            // Modal overlay for mint form
            <Show when=move || show_mint_form.get()>
                <div class="modal-overlay" on:click=move |_| set_show_mint_form.set(false)>
                    <div class="modal-content" on:click=|e| e.stop_propagation()>
                        <div class="modal-header">
                            <h3>"Engrave Memories & Mint MEMO Tokens"</h3>
                            <button 
                                class="modal-close-btn"
                                on:click=move |_| set_show_mint_form.set(false)
                                title="Close"
                            >
                                "×"
                            </button>
                        </div>
                        
                        <div class="modal-body">
                            // Use the new MintForm component
                            {
                                let success_cb = on_mint_success.clone();
                                let error_cb = on_mint_error.clone();
                                
                                view! {
                                    <MintForm 
                                        session=session 
                                        on_mint_success=success_cb
                                        on_mint_error=error_cb
                                    />
                                }
                            }
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
