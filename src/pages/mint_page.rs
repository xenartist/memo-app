use leptos::*;
use crate::core::session::Session;
use crate::core::storage_mint::{get_mint_storage, MintRecord};
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use crate::pages::mint_form::MintForm;
use crate::pages::memo_card::MemoCard;
use std::rc::Rc;

#[component]
pub fn MintPage(
    session: RwSignal<Session>
) -> impl IntoView {
    // add storage status display
    let (storage_status, set_storage_status) = create_signal(String::new());
    
    // add signal to control mint form visibility
    let (show_mint_form, set_show_mint_form) = create_signal(false);
    
    // pagination related signals
    let (all_mint_records, set_all_mint_records) = create_signal(Vec::<MintRecord>::new());
    let (current_page, set_current_page) = create_signal(1usize);
    let (page_size, _set_page_size) = create_signal(10usize); // show 10 cards per page
    let (total_records, set_total_records) = create_signal(0usize);
    let (is_loading_records, set_is_loading_records) = create_signal(false);
    let (records_error, set_records_error) = create_signal(String::new());
    
    // add a signal to control whether to start loading memo cards
    let (should_load_cards, set_should_load_cards) = create_signal(false);

    // calculate records for current page - avoid using create_memo, calculate directly during rendering
    let get_current_page_records = move || {
        let all_records = all_mint_records.get();
        let page = current_page.get();
        let size = page_size.get();
        let start_idx = (page - 1) * size;
        let end_idx = std::cmp::min(start_idx + size, all_records.len());
        
        if start_idx < all_records.len() {
            all_records[start_idx..end_idx].to_vec()
        } else {
            Vec::new()
        }
    };

    // calculate total pages
    let get_total_pages = move || {
        let total = total_records.get();
        let size = page_size.get();
        if total == 0 {
            1
        } else {
            ((total as f64) / (size as f64)).ceil() as usize
        }
    };

    // load mint records function - load all data once
    let load_mint_records = move || {
        set_is_loading_records.set(true);
        set_records_error.set(String::new());
        
        spawn_local(async move {
            TimeoutFuture::new(100).await;
            
            match get_mint_storage().get_all_records().await {
                Ok(records) => {
                    let total_count = records.len();
                    set_all_mint_records.set(records);
                    set_total_records.set(total_count);
                    log::info!("Successfully loaded {} mint records", total_count);
                }
                Err(e) => {
                    let error_msg = format!("Failed to load mint records: {}", e);
                    set_records_error.set(error_msg.clone());
                    log::error!("{}", error_msg);
                }
            }
            
            set_is_loading_records.set(false);
        });
    };

    // pagination control functions
    let go_to_page = move |page: usize| {
        let max_page = get_total_pages();
        if page >= 1 && page <= max_page {
            set_current_page.set(page);
            
            // scroll to top of page
            if let Some(window) = web_sys::window() {
                window.scroll_to_with_x_and_y(0.0, 0.0);
            }
        }
    };

    let handle_prev_page = move |_| {
        let current = current_page.get();
        if current > 1 {
            go_to_page(current - 1);
        }
    };

    let handle_next_page = move |_| {
        let current = current_page.get();
        let max_page = get_total_pages();
        if current < max_page {
            go_to_page(current + 1);
        }
    };

    // get storage status on initialization (but not immediately load memo cards)
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
        
        // delay starting memo cards loading (let page render first)
        spawn_local(async move {
            // delay 800ms to let page fully render before starting to load memo cards
            TimeoutFuture::new(800).await;
            set_should_load_cards.set(true);
        });
    });

    // only start loading records when should_load_cards is true
    create_effect(move |_| {
        if should_load_cards.get() {
            load_mint_records();
        }
    });

    // Optional callbacks for mint events
    let on_mint_success = Rc::new(move |_signature: String, tokens_minted: u64, total_minted: u64| {
        log::info!("Mint successful on page level: {} tokens minted, total: {}", tokens_minted, total_minted);
        
        // Update storage status after successful mint
        spawn_local(async move {
            if let Ok(status) = get_mint_storage().get_detailed_storage_status().await {
                set_storage_status.set(status);
            }
        });
        
        // Reload mint records to show the new one
        load_mint_records();
        // jump to first page to show latest records
        set_current_page.set(1);
    });

    let on_mint_error = Rc::new(move |error: String| {
        log::error!("Mint error on page level: {}", error);
    });

    // refresh records manually
    let handle_refresh_records = move |_| {
        load_mint_records();
        set_current_page.set(1); // reset to first page
    };

    // parse memo JSON to extract title and image
    let parse_memo_json = |memo_json: &str| -> (Option<String>, Option<String>) {
        match serde_json::from_str::<serde_json::Value>(memo_json) {
            Ok(memo) => {
                let title = memo.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
                let image = memo.get("image").and_then(|v| v.as_str()).map(|s| s.to_string());
                (title, image)
            }
            Err(_) => (None, None)
        }
    };

    view! {
        <div class="mint-page">
            <div class="mint-page-header">
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
            
            // Main content area - Mint Records List
            <div class="mint-content">
                <div class="header-section" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                    <h2>
                        "My Latest Mint"
                        // show pagination info
                        <Show when=move || { total_records.get() > 0 }>
                            <span style="font-size: 0.8em; color: #666; margin-left: 10px;">
                                {move || format!("({} records)", total_records.get())}
                            </span>
                        </Show>
                    </h2>
                    <button 
                        class="refresh-btn"
                        on:click=handle_refresh_records
                        prop:disabled=move || is_loading_records.get()
                    >
                        {move || if is_loading_records.get() {
                            view! {
                                <>
                                    <i class="fas fa-sync-alt fa-spin"></i>
                                    " Refreshing..."
                                </>
                            }
                        } else {
                            view! {
                                <>
                                    <i class="fas fa-sync-alt"></i>
                                    " Refresh"
                                </>
                            }
                        }}
                    </button>
                </div>

                // error message display
                {move || {
                    let error = records_error.get();
                    if !error.is_empty() {
                        view! {
                            <div class="error-banner" style="margin-bottom: 16px; padding: 12px; background: #fff3cd; border: 1px solid #ffc107; border-radius: 4px; color: #856404;">
                                <i class="fas fa-exclamation-triangle" style="margin-right: 8px;"></i>
                                {error}
                            </div>
                        }
                    } else {
                        view! { <div></div> }
                    }
                }}

                {move || {
                    let all_records = all_mint_records.get();
                    let is_loading = is_loading_records.get();
                    let should_load = should_load_cards.get();
                    
                    if !should_load {
                        // page just loaded, show loading message instead of loading
                        view! {
                            <div class="welcome-state" style="text-align: center; padding: 3rem 2rem; color: #666;">
                                <div style="font-size: 1.2rem; margin-bottom: 1rem;">
                                    <i class="fas fa-coins" style="margin-right: 8px; color: #28a745;"></i>
                                    "Welcome to Your Mint History"
                                </div>
                                <p>"Loading your memories..."</p>
                            </div>
                        }
                    } else if all_records.is_empty() && is_loading {
                        // show loading when loading and no data
                        view! {
                            <div class="loading-container">
                                <div class="loading-spinner"></div>
                                <p class="loading-text">"Loading your mint history..."</p>
                            </div>
                        }
                    } else if all_records.is_empty() && !is_loading {
                        // no data and not loading
                        view! {
                            <div class="empty-state">
                                <p class="empty-message">
                                    <i class="fas fa-coins" style="margin-right: 8px;"></i>
                                    "No mint records found. Start by creating your first memory!"
                                </p>
                            </div>
                        }
                    } else {
                        // show cards when there is data
                        view! {
                            <div class="records-container">
                                // pagination controls (top)
                                <Show when=move || { get_total_pages() > 1 }>
                                    <div class="pagination-top" style="display: flex; justify-content: center; align-items: center; margin-bottom: 20px; gap: 10px;">
                                        <button 
                                            class="pagination-btn"
                                            on:click=handle_prev_page
                                            disabled=move || current_page.get() == 1
                                            style="padding: 8px 12px; border: 1px solid #ddd; background: white; cursor: pointer; border-radius: 4px;"
                                        >
                                            "← Previous"
                                        </button>
                                        
                                        <span class="pagination-info" style="margin: 0 15px; font-size: 0.9em; color: #666;">
                                            {move || format!("Page {} of {}", current_page.get(), get_total_pages())}
                                        </span>
                                        
                                        <button 
                                            class="pagination-btn"
                                            on:click=handle_next_page
                                            disabled=move || { current_page.get() >= get_total_pages() }
                                            style="padding: 8px 12px; border: 1px solid #ddd; background: white; cursor: pointer; border-radius: 4px;"
                                        >
                                            "Next →"
                                        </button>
                                    </div>
                                </Show>

                                // memo cards - only show current page data
                                <div class="memo-cards">
                                    <For
                                        each=move || get_current_page_records()
                                        key=|record| format!("{}_{}", record.timestamp as i64, record.signature)
                                        children=move |record| {
                                            // get current user's pubkey address
                                            let user_pubkey = session.get().get_public_key().unwrap_or_else(|_| "Unknown".to_string());
                                            
                                            // format user pubkey (display first 4 and last 4 characters)
                                            let display_pubkey = if user_pubkey.len() >= 8 && user_pubkey != "Unknown" {
                                                format!("{}...{}", &user_pubkey[..4], &user_pubkey[user_pubkey.len()-4..])
                                            } else {
                                                user_pubkey
                                            };
                                            
                                            // format signature (display first 8 and last 8 characters)
                                            let display_signature = if record.signature.len() >= 16 {
                                                format!("{}...{}", &record.signature[..8], &record.signature[record.signature.len()-8..])
                                            } else {
                                                record.signature.clone()
                                            };
                                            
                                            // parse memo JSON to get title and image
                                            let (title, image) = parse_memo_json(&record.memo_json);
                                            
                                            // convert timestamp (milliseconds) to seconds for blocktime format
                                            let blocktime = (record.timestamp / 1000.0) as i64;
                                            
                                            // handle title and image, convert to String type
                                            let final_title = title.unwrap_or_else(|| "Memory".to_string());
                                            let final_image = image.clone().unwrap_or_else(|| {
                                                // default placeholder image for mint records
                                                "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNjQiIGhlaWdodD0iNjQiIHZpZXdCb3g9IjAgMCA2NCA2NCIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3Qgd2lkdGg9IjY0IiBoZWlnaHQ9IjY0IiBmaWxsPSIjZTZmN2ZmIi8+Cjx0ZXh0IHg9IjMyIiB5PSIzNiIgdGV4dC1hbmNob3I9Im1pZGRsZSIgZm9udC1mYW1pbHk9IkFyaWFsIiBmb250LXNpemU9IjEyIiBmaWxsPSIjNGY4NmY3Ij5NaW50PC90ZXh0Pgo8L3N2Zz4K".to_string()
                                            });
                                            
                                            view! {
                                                <MemoCard
                                                    title=final_title
                                                    image=final_image
                                                    signature=display_signature
                                                    pubkey=display_pubkey
                                                    blocktime=blocktime
                                                />
                                            }
                                        }
                                    />
                                </div>

                                // pagination controls (bottom)
                                <Show when=move || { get_total_pages() > 1 }>
                                    <div class="pagination-bottom" style="display: flex; justify-content: center; align-items: center; margin-top: 30px; gap: 10px;">
                                        <button 
                                            class="pagination-btn"
                                            on:click=handle_prev_page
                                            disabled=move || current_page.get() == 1
                                            style="padding: 8px 12px; border: 1px solid #ddd; background: white; cursor: pointer; border-radius: 4px;"
                                        >
                                            "← Previous"
                                        </button>
                                        
                                        // page number quick jump
                                        <div style="display: flex; gap: 5px; align-items: center;">
                                            {move || {
                                                let current = current_page.get();
                                                let total = get_total_pages();
                                                let mut pages = Vec::new();
                                                
                                                // page display logic: always show page 1, pages around current, and last page
                                                let start = if current <= 3 { 1 } else { current - 2 };
                                                let end = if current + 2 >= total { total } else { current + 2 };
                                                
                                                for page in start..=end {
                                                    pages.push(page);
                                                }
                                                
                                                pages.into_iter().map(move |page| {
                                                    let is_current = page == current;
                                                    view! {
                                                        <button
                                                            class="page-number-btn"
                                                            on:click=move |_| go_to_page(page)
                                                            style=move || format!(
                                                                "padding: 6px 10px; border: 1px solid #ddd; cursor: pointer; border-radius: 4px; {}",
                                                                if is_current { "background: #007bff; color: white;" } else { "background: white;" }
                                                            )
                                                        >
                                                            {page.to_string()}
                                                        </button>
                                                    }
                                                }).collect::<Vec<_>>()
                                            }}
                                        </div>
                                        
                                        <span class="pagination-info" style="margin: 0 15px; font-size: 0.9em; color: #666;">
                                            {move || format!("Page {} of {}", current_page.get(), get_total_pages())}
                                        </span>
                                        
                                        <button 
                                            class="pagination-btn"
                                            on:click=handle_next_page
                                            disabled=move || { current_page.get() >= get_total_pages() }
                                            style="padding: 8px 12px; border: 1px solid #ddd; background: white; cursor: pointer; border-radius: 4px;"
                                        >
                                            "Next →"
                                        </button>
                                    </div>
                                </Show>
                            </div>
                        }
                    }
                }}
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
