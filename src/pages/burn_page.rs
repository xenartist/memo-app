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

    // ✅ add reset related state
    let (is_resetting, set_is_resetting) = create_signal(false);
    let (show_reset_confirm, set_show_reset_confirm) = create_signal(false);
    let (reset_message, set_reset_message) = create_signal(String::new());

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

    // ✅ add reset storage function
    let handle_reset_storage = move |_| {
        set_show_reset_confirm.set(true);
    };
    
    // ✅ confirm reset function
    let handle_confirm_reset = move |_| {
        set_is_resetting.set(true);
        set_show_reset_confirm.set(false);
        set_reset_message.set(String::new());
        
        spawn_local(async move {
            let burn_storage = get_burn_storage();
            match burn_storage.clear_all_records().await {
                Ok(_) => {
                    log::info!("✅ Successfully cleared all burn records");
                    set_reset_message.set("✅ Storage cleared successfully! All burn records have been deleted.".to_string());
                    
                    // 清空当前页面的记录显示
                    set_all_burn_records.set(Vec::new());
                    set_total_records.set(0);
                    set_current_page.set(1);
                    set_records_error.set(String::new());
                }
                Err(e) => {
                    log::error!("❌ Failed to clear burn records: {}", e);
                    set_reset_message.set(format!("❌ Failed to clear storage: {}", e));
                }
            }
            set_is_resetting.set(false);
        });
    };
    
    // ✅ cancel reset function
    let handle_cancel_reset = move |_| {
        set_show_reset_confirm.set(false);
    };

    // ✅ update parse_memo_json function name, specifically parse mint_memo_json
    fn parse_mint_memo_json(mint_memo_json: &str) -> (Option<String>, Option<String>, Option<String>) {
        match serde_json::from_str::<serde_json::Value>(mint_memo_json) {
            Ok(json) => {
                let title = json.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
                let image = json.get("image").and_then(|v| v.as_str()).map(|s| s.to_string());
                let content = json.get("content").and_then(|v| v.as_str()).map(|s| s.to_string());
                (title, image, content)
            }
            Err(e) => {
                log::warn!("Failed to parse mint memo JSON: {}", e);
                (None, None, None)
            }
        }
    }

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
                    <div style="display: flex; gap: 10px; align-items: center;">
                        // Refresh button
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
                        
                        // ✅ Reset Storage button
                        <button 
                            class="reset-storage-btn"
                            style="background: #dc3545; color: white; border: 1px solid #dc3545; padding: 8px 12px; border-radius: 4px; cursor: pointer; font-size: 0.9em;"
                            on:click=handle_reset_storage
                            prop:disabled=move || is_resetting.get() || is_loading_records.get()
                            title="Clear all burn records from local storage"
                        >
                            {move || if is_resetting.get() {
                                view! {
                                    <>
                                        <i class="fas fa-spinner fa-spin"></i>
                                        " Clearing..."
                                    </>
                                }
                            } else {
                                view! {
                                    <>
                                        <i class="fas fa-trash-alt"></i>
                                        " Reset Storage"
                                    </>
                                }
                            }}
                        </button>
                    </div>
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

                // ✅ Reset success/error message display
                <Show when=move || !reset_message.get().is_empty()>
                    <div class="reset-message" style="margin-bottom: 16px; padding: 12px; border-radius: 4px;"
                         class:success=move || reset_message.get().starts_with("✅")
                         class:error=move || reset_message.get().starts_with("❌")
                         style:background=move || if reset_message.get().starts_with("✅") { "#d4edda" } else { "#f8d7da" }
                         style:border=move || if reset_message.get().starts_with("✅") { "1px solid #28a745" } else { "1px solid #dc3545" }
                         style:color=move || if reset_message.get().starts_with("✅") { "#155724" } else { "#721c24" }
                    >
                        {reset_message}
                    </div>
                </Show>

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
                                    "No burn records found. Start by burning your first MEMO!"
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
                                            
                                            // ✅ use new field name to parse mint memo JSON
                                            let (title, image, content) = parse_mint_memo_json(&record.mint_memo_json);
                                            
                                            // convert timestamp (milliseconds) to seconds for blocktime format
                                            let blocktime = (record.timestamp / 1000.0) as i64;
                                            
                                            // ✅ convert amount to tokens
                                            let amount_tokens = record.amount as f64 / 1_000_000_000.0;
                                            
                                            // handle title and image, convert to String type
                                            let final_title = title.clone().unwrap_or_else(|| format!("Burned {} MEMO", amount_tokens));
                                            
                                            // ✅ use real pixel art data, not placeholder SVG
                                            let final_image = image.clone().unwrap_or_else(|| {
                                                // if no image data, use simple placeholder, not complex SVG
                                                "".to_string()  // empty string will make MemoCard show default placeholder
                                            });
                                            
                                            view! {
                                                <MemoCard
                                                    title=final_title
                                                    image=final_image  // ✅ here pass the pixel art encoded string
                                                    content=content.unwrap_or_else(|| "".to_string())
                                                    signature=display_signature
                                                    pubkey="Burned".to_string()
                                                    blocktime=blocktime
                                                    amount=amount_tokens  // ✅ use converted token amount
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
                session=session
                on_close=Callback::new(move |_| {
                    log::info!("Burn details modal closed");
                })
            />

            // ✅ Reset confirm dialog
            <Show when=move || show_reset_confirm.get()>
                <div class="modal-overlay" style="position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); z-index: 1000; display: flex; justify-content: center; align-items: center;">
                    <div class="reset-confirm-modal" style="background: white; border-radius: 8px; padding: 24px; max-width: 400px; width: 90%; box-shadow: 0 4px 12px rgba(0,0,0,0.3);">
                        <div class="modal-header" style="text-align: center; margin-bottom: 20px;">
                            <i class="fas fa-exclamation-triangle" style="font-size: 2em; color: #dc3545; margin-bottom: 10px;"></i>
                            <h3 style="margin: 0; color: #dc3545;">"Confirm Reset Storage"</h3>
                        </div>
                        
                        <div class="modal-body" style="margin-bottom: 24px;">
                            <p style="margin: 0 0 16px 0; text-align: center; color: #333;">
                                "Are you sure you want to clear all burn records from local storage?"
                            </p>
                            <div style="background: #f8f9fa; border: 1px solid #dee2e6; border-radius: 4px; padding: 12px; font-size: 0.9em; color: #6c757d;">
                                <strong>"Warning:"</strong>
                                <ul style="margin: 8px 0 0 0; padding-left: 20px;">
                                    <li>"This action cannot be undone"</li>
                                    <li>"All locally stored burn history will be permanently deleted"</li>
                                    <li>"This only affects local storage, not blockchain records"</li>
                                </ul>
                            </div>
                        </div>
                        
                        <div class="modal-footer" style="display: flex; gap: 12px; justify-content: center;">
                            <button 
                                class="cancel-btn"
                                style="background: #6c757d; color: white; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer;"
                                on:click=handle_cancel_reset
                            >
                                <i class="fas fa-times"></i>
                                " Cancel"
                            </button>
                            <button 
                                class="confirm-btn"
                                style="background: #dc3545; color: white; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer;"
                                on:click=handle_confirm_reset
                            >
                                <i class="fas fa-trash-alt"></i>
                                " Clear All Records"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
} 