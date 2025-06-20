use leptos::*;
use crate::core::session::Session;
use crate::core::storage_burn::{get_burn_storage, BurnRecord};
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use crate::pages::burn_form::BurnForm;
use crate::pages::memo_card::{MemoCard, MemoDetails};
use crate::pages::memo_card_details::MemoCardDetails;
use std::rc::Rc;

#[component]
pub fn BurnPage(
    session: RwSignal<Session>
) -> impl IntoView {
    // add storage status display
    let (storage_status, set_storage_status) = create_signal(String::new());
    
    // add signal to control burn form visibility
    let (show_burn_form, set_show_burn_form) = create_signal(false);
    
    // add signal to control details modal visibility
    let (show_details_modal, set_show_details_modal) = create_signal(false);
    let (current_memo_details, set_current_memo_details) = create_signal(Option::<MemoDetails>::None);
    
    // pagination related signals
    let (all_burn_records, set_all_burn_records) = create_signal(Vec::<BurnRecord>::new());
    let (current_page, set_current_page) = create_signal(1usize);
    let (page_size, _set_page_size) = create_signal(10usize); // show 10 cards per page
    let (total_records, set_total_records) = create_signal(0usize);
    let (is_loading_records, set_is_loading_records) = create_signal(false);
    let (records_error, set_records_error) = create_signal(String::new());
    
    // add a signal to control whether to start loading memo cards
    let (should_load_cards, set_should_load_cards) = create_signal(false);

    // calculate records for current page
    let get_current_page_records = move || {
        let all_records = all_burn_records.get();
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

    // load burn records function
    let load_burn_records = move || {
        set_is_loading_records.set(true);
        set_records_error.set(String::new());
        
        spawn_local(async move {
            TimeoutFuture::new(100).await;
            
            match get_burn_storage().get_all_records().await {
                Ok(records) => {
                    let total_count = records.len();
                    set_all_burn_records.set(records);
                    set_total_records.set(total_count);
                    log::info!("Successfully loaded {} burn records", total_count);
                }
                Err(e) => {
                    let error_msg = format!("Failed to load burn records: {}", e);
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

    let handle_prev_page = move |_: web_sys::MouseEvent| {
        let current = current_page.get();
        if current > 1 {
            go_to_page(current - 1);
        }
    };

    let handle_next_page = move |_: web_sys::MouseEvent| {
        let current = current_page.get();
        let max_page = get_total_pages();
        if current < max_page {
            go_to_page(current + 1);
        }
    };

    // get storage status on initialization
    create_effect(move |_| {
        spawn_local(async move {
            log::info!("=== Burn Storage Initialization Start ===");
            
            // add small delay to ensure DOM is ready
            TimeoutFuture::new(100).await;
            
            // first try to get sync status
            match get_burn_storage().get_storage_status() {
                Ok(basic_status) => {
                    log::info!("Basic burn storage status: {}", basic_status);
                    set_storage_status.set(basic_status);
                    
                    // then try to get detailed status
                    match get_burn_storage().get_detailed_storage_status().await {
                        Ok(detailed_status) => {
                            log::info!("Detailed burn storage status: {}", detailed_status);
                            set_storage_status.set(detailed_status);
                        },
                        Err(e) => {
                            log::error!("Failed to get detailed burn storage status: {}", e);
                            // keep basic status display
                        }
                    }
                },
                Err(e) => {
                    log::error!("Failed to get basic burn storage status: {}", e);
                    set_storage_status.set(format!("Storage Error: {}", e));
                }
            }
            
            log::info!("=== Burn Storage Initialization End ===");
        });
        
        // delay starting memo cards loading
        spawn_local(async move {
            // delay 800ms to let page fully render before starting to load memo cards
            TimeoutFuture::new(800).await;
            set_should_load_cards.set(true);
        });
    });

    // only start loading records when should_load_cards is true
    create_effect(move |_| {
        if should_load_cards.get() {
            load_burn_records();
        }
    });

    // Optional callbacks for burn events - 改为 Callback 类型
    let on_burn_success = Callback::new(move |data: (String, u64)| {
        let (signature, tokens_burned) = data;
        log::info!("Burn successful on page level: {} tokens burned", tokens_burned);
        
        // Update storage status after successful burn
        spawn_local(async move {
            if let Ok(status) = get_burn_storage().get_detailed_storage_status().await {
                set_storage_status.set(status);
            }
        });
        
        // Reload burn records to show the new one
        load_burn_records();
        // jump to first page to show latest records
        set_current_page.set(1);
        // close the burn form modal
        set_show_burn_form.set(false);
    });

    let on_burn_error = Callback::new(move |error: String| {
        log::error!("Burn error on page level: {}", error);
    });

    // refresh records manually
    let handle_refresh_records = move |_: web_sys::MouseEvent| {
        load_burn_records();
        set_current_page.set(1); // reset to first page
    };

    // parse memo JSON to extract title and image
    let parse_memo_json = |memo_json: &str| -> (Option<String>, Option<String>, Option<String>) {
        match serde_json::from_str::<serde_json::Value>(memo_json) {
            Ok(memo) => {
                let title = memo.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
                let image = memo.get("image").and_then(|v| v.as_str()).map(|s| s.to_string());
                let content = memo.get("content").and_then(|v| v.as_str()).map(|s| s.to_string());
                (title, image, content)
            }
            Err(_) => (None, None, None)
        }
    };

    view! {
        <div class="burn-page">
            <div class="burn-page-header">
                // Action buttons
                <div class="burn-actions">
                    <button 
                        class="open-burn-form-btn"
                        on:click=move |_| set_show_burn_form.set(true)
                        disabled=move || !session.get().has_user_profile()
                    >
                        "🔥 Burn MEMO"
                    </button>
                    
                    // Show warning when no profile
                    <Show when=move || !session.get().has_user_profile()>
                        <div class="no-profile-warning">
                            <p>"⚠️ Please create your profile in the Profile page before you can start burning."</p>
                        </div>
                    </Show>
                </div>
            </div>
            
            // Main content area - Burn Records List
            <div class="burn-content">
                <div class="header-section" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                    <h2>
                        "My Burn History"
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
                    let all_records = all_burn_records.get();
                    let is_loading = is_loading_records.get();
                    let should_load = should_load_cards.get();
                    
                    if !should_load {
                        // page just loaded, show loading message
                        view! {
                            <div class="welcome-state" style="text-align: center; padding: 3rem 2rem; color: #666;">
                                <div style="font-size: 1.2rem; margin-bottom: 1rem;">
                                    <i class="fas fa-fire" style="margin-right: 8px; color: #dc3545;"></i>
                                    "Welcome to Your Burn History"
                                </div>
                                <p>"Loading your burn records..."</p>
                            </div>
                        }
                    } else if all_records.is_empty() && is_loading {
                        // show loading when loading and no data
                        view! {
                            <div class="loading-container">
                                <div class="loading-spinner"></div>
                                <p class="loading-text">"Loading your burn history..."</p>
                            </div>
                        }
                    } else if all_records.is_empty() && !is_loading {
                        // no data and not loading
                        view! {
                            <div class="empty-state">
                                <p class="empty-message">
                                    <i class="fas fa-fire" style="margin-right: 8px;"></i>
                                    "No burn records found. Start by burning your first MEMO token!"
                                </p>
                            </div>
                        }
                    } else {
                        // show cards when there is data
                        view! {
                            <div class="records-container">
                                // memo cards - only show current page data
                                <div class="memo-cards">
                                    <For
                                        each=move || get_current_page_records()
                                        key=|record| format!("{}_{}", record.timestamp as i64, record.signature)
                                        children=move |record| {
                                            // format signature (display first 8 and last 8 characters)
                                            let display_signature = if record.signature.len() >= 16 {
                                                format!("{}...{}", &record.signature[..8], &record.signature[record.signature.len()-8..])
                                            } else {
                                                record.signature.clone()
                                            };
                                            
                                            // parse memo JSON to get title, image, and content
                                            let (title, image, content) = parse_memo_json(&record.memo_json);
                                            
                                            // convert timestamp (milliseconds) to seconds for blocktime format
                                            let blocktime = (record.timestamp / 1000.0) as i64;
                                            
                                            // handle title and image, convert to String type
                                            let final_title = title.clone().unwrap_or_else(|| format!("Burned {} MEMO", record.amount));
                                            let final_image = image.clone().unwrap_or_else(|| {
                                                // default placeholder image for burn records
                                                "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNjQiIGhlaWdodD0iNjQiIHZpZXdCb3g9IjAgMCA2NCA2NCIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3Qgd2lkdGg9IjY0IiBoZWlnaHQ9IjY0IiBmaWxsPSIjZmZlNmU2Ii8+Cjx0ZXh0IHg9IjMyIiB5PSIzNiIgdGV4dC1hbmNob3I9Im1pZGRsZSIgZm9udC1mYW1pbHk9IkFyaWFsIiBmb250LXNpemU9IjEyIiBmaWxsPSIjZGMzNTQ1Ij5CdXJuPC90ZXh0Pgo8L3N2Zz4K".to_string()
                                            });
                                            
                                            view! {
                                                <MemoCard
                                                    title=final_title
                                                    image=final_image
                                                    content=content.unwrap_or_else(|| "".to_string())
                                                    signature=display_signature
                                                    pubkey="Burned".to_string()
                                                    blocktime=blocktime
                                                    amount=record.amount as f64
                                                    class="burned-memo-card"
                                                    on_details_click=Callback::new(move |details: MemoDetails| {
                                                        log::info!("Details clicked for burned signature: {}", details.signature);
                                                        set_current_memo_details.set(Some(details));
                                                        set_show_details_modal.set(true);
                                                    })
                                                    // Note: no on_burn_click for already burned items
                                                />
                                            }
                                        }
                                    />
                                </div>
                            </div>
                        }
                    }
                }}
            </div>
            
            // Modal overlay for burn form
            <Show when=move || show_burn_form.get()>
                <div class="modal-overlay" on:click=move |_| set_show_burn_form.set(false)>
                    <div class="modal-content" on:click=|e| e.stop_propagation()>
                        <div class="modal-header">
                            <h3>"Burn MEMO"</h3>
                            <button 
                                class="modal-close-btn"
                                on:click=move |_| set_show_burn_form.set(false)
                                title="Close"
                            >
                                "×"
                            </button>
                        </div>
                        
                        <div class="modal-body">
                            {
                                let success_cb = on_burn_success.clone();
                                let error_cb = on_burn_error.clone();
                                
                                view! {
                                    <BurnForm 
                                        session=session 
                                        on_burn_success=success_cb
                                        on_burn_error=error_cb
                                    />
                                }
                            }
                        </div>
                    </div>
                </div>
            </Show>

            // use details modal component (without burn button for already burned items)
            <MemoCardDetails 
                show_modal=show_details_modal.into()
                set_show_modal=set_show_details_modal
                memo_details=current_memo_details.into()
                // Note: no on_burn_click for already burned items
                on_close=Callback::new(move |_| {
                    log::info!("Burn details modal closed");
                })
            />
        </div>
    }
} 