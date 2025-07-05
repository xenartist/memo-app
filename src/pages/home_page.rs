use leptos::*;
use crate::core::cache::{BurnRecord, get_latest_burn_shard, refresh_latest_burn_shard};
use crate::pages::memo_card::{MemoCard, MemoDetails};
use crate::pages::memo_card_details::MemoCardDetails;
use crate::pages::burn_onchain::{BurnOnchain, BurnOptions};
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_futures::spawn_local;
use crate::core::rpc_base::RpcConnection;
use crate::core::session::Session;

// BurnRecordWithImage component, handle async memo loading
#[component]
pub fn BurnRecordWithImage(
    burn_record: BurnRecord,
    session: RwSignal<Session>,
) -> impl IntoView {
    let (memo_data, set_memo_data) = create_signal(None::<(Option<String>, Option<String>, Option<String>)>);
    let (is_loading_memo, set_is_loading_memo) = create_signal(true);
    
    // ✅ clone values needed before create_effect
    let signature_for_effect = burn_record.signature.clone();
    let signature_for_display = burn_record.signature.clone();
    let pubkey_for_display = burn_record.pubkey.clone();
    let blocktime = burn_record.blocktime;
    let amount = burn_record.amount;
    
    // ✅ async load memo data effect - use cloned signature
    create_effect(move |_| {
        let signature = signature_for_effect.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let rpc = RpcConnection::new();
            match rpc.get_transaction_memo(&signature).await {
                Ok(Some(memo_info)) => {
                    let (title, image, content) = parse_memo_json(&memo_info.memo);
                    set_memo_data.set(Some((title, image, content)));
                    set_is_loading_memo.set(false);
                }
                Ok(None) => {
                    log::warn!("No memo found for signature: {}", signature);
                    set_memo_data.set(Some((None, None, None)));
                    set_is_loading_memo.set(false);
                }
                Err(e) => {
                    log::error!("Failed to load memo for signature {}: {}", signature, e);
                    set_memo_data.set(Some((None, None, None)));
                    set_is_loading_memo.set(false);
                }
            }
        });
    });
    
    // ✅ use pre-cloned values
    let display_signature = if signature_for_display.len() >= 16 {
        format!("{}...{}", &signature_for_display[..8], &signature_for_display[signature_for_display.len()-8..])
    } else {
        signature_for_display.clone()
    };
    let display_pubkey = if pubkey_for_display.len() >= 16 {
        format!("{}...{}", &pubkey_for_display[..8], &pubkey_for_display[pubkey_for_display.len()-8..])
    } else {
        pubkey_for_display.clone()
    };
    
    let amount_tokens = amount as f64 / 1_000_000_000.0;

    // MemoCardDetails modal states
    let (show_details_modal, set_show_details_modal) = create_signal(false);
    let (current_memo_details, set_current_memo_details) = create_signal(None::<MemoDetails>);
    
    // BurnOnchain modal states  
    let (show_burn_onchain, set_show_burn_onchain) = create_signal(false);
    let (burn_signature, set_burn_signature) = create_signal(String::new());
    
    // ✅ define callbacks outside view to avoid nested move closure
    let on_details_callback = {
        let set_current_memo_details = set_current_memo_details.clone();
        let set_show_details_modal = set_show_details_modal.clone();
        Callback::new(move |details: MemoDetails| {
            log::info!("Details clicked for burn signature: {}", details.signature);
            set_current_memo_details.set(Some(details));
            set_show_details_modal.set(true);
        })
    };
    
    let on_burn_callback = {
        let signature_for_burn = signature_for_display.clone();
        let display_pubkey_clone = display_pubkey.clone();
        let set_burn_signature = set_burn_signature.clone();
        let set_current_memo_details = set_current_memo_details.clone();
        let set_show_burn_onchain = set_show_burn_onchain.clone();
        
        Callback::new(move |signature: String| {
            log::info!("Burn clicked for signature: {}", signature);
            
            set_burn_signature.set(signature.clone());
            
            // get current memo data
            let (title, image, content) = memo_data.get_untracked().unwrap_or((None, None, None));
            let memo_details = MemoDetails {
                title,
                image,
                content,
                signature: signature_for_burn.clone(),
                burn_signature: None,
                pubkey: display_pubkey_clone.clone(),
                blocktime,
                amount: Some(amount_tokens),
            };
            set_current_memo_details.set(Some(memo_details));
            
            set_show_burn_onchain.set(true);
        })
    };
    
    view! {
        <>
            {move || {
                let (title, image, content) = memo_data.get().unwrap_or((None, None, None));
                
                // if still loading, show loading card
                if is_loading_memo.get() {
                    view! {
                        <div class="memo-card loading">
                            <div class="memo-header">
                                <h4 class="memo-title">"Loading..."</h4>
                            </div>
                            <div class="memo-image-container">
                                <div class="memo-image-placeholder">
                                    <i class="fas fa-spinner fa-spin"></i>
                                    <span>"Loading memo..."</span>
                                </div>
                            </div>
                            <div class="memo-info">
                                <div class="memo-info-item">
                                    <span class="label">"Signature:"</span>
                                    <span class="value signature">{display_signature.clone()}</span>
                                </div>
                                <div class="memo-info-item">
                                    <span class="label">"From:"</span>
                                    <span class="value pubkey">{display_pubkey.clone()}</span>
                                </div>
                            </div>
                        </div>
                    }.into_view()
                } else {
                    // loading completed, show real MemoCard
                    view! {
                        <MemoCard
                            title=title.unwrap_or_else(|| "Burn Memory".to_string())
                            image=image.unwrap_or_else(|| {
                                // default burn placeholder image
                                "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNjQiIGhlaWdodD0iNjQiIHZpZXdCb3g9IjAgMCA2NCA2NCIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3Qgd2lkdGg9IjY0IiBoZWlnaHQ9IjY0IiBmaWxsPSIjZmZlNmU2Ii8+Cjx0ZXh0IHg9IjMyIiB5PSIzNiIgdGV4dC1hbmNob3I9Im1pZGRsZSIgZm9udC1mYW1pbHk9IkFyaWFsIiBmb250LXNpemU9IjEyIiBmaWxsPSIjZGM2MjY4Ij5CdXJuPC90ZXh0Pgo8L3N2Zz4K".to_string()
                            })
                            content=content.unwrap_or_else(|| "".to_string())
                            signature=signature_for_display.clone()
                            pubkey=display_pubkey.clone()
                            blocktime=blocktime
                            amount=amount_tokens
                            on_details_click=on_details_callback  // ✅ use predefined callback
                            on_burn_click=on_burn_callback        // ✅ use predefined callback
                        />
                    }.into_view()
                }
            }}
            
            // MemoCardDetails Modal
            <MemoCardDetails 
                show_modal=show_details_modal.into()
                set_show_modal=set_show_details_modal
                memo_details=current_memo_details.into()
                session=session
                on_burn_choice=Callback::new(move |(signature, burn_options): (String, BurnOptions)| {
                    log::info!("Burn choice made from home page for signature: {}, options: {:?}", signature, burn_options);
                    // TODO: implement burn processing logic
                })
                on_close=Callback::new(move |_| {
                    log::info!("Details modal closed from home page");
                })
            />
            
            // BurnOnchain Modal
            <BurnOnchain
                show_modal=show_burn_onchain.into()
                set_show_modal=set_show_burn_onchain
                signature=burn_signature.into()
                memo_details=current_memo_details.into()
                session=session
                on_burn_choice=Callback::new(move |(sig, burn_options): (String, BurnOptions)| {
                    log::info!("Burn onchain choice made for signature: {}, options: {:?}", sig, burn_options);
                    // TODO: implement burn processing logic
                })
                on_close=Callback::new(move |_| {
                    log::info!("Burn onchain modal closed");
                    set_show_burn_onchain.set(false);
                })
            />
        </>
    }
}

// parse memo JSON, extract title, image, content
fn parse_memo_json(memo_json: &str) -> (Option<String>, Option<String>, Option<String>) {
    match serde_json::from_str::<serde_json::Value>(memo_json) {
        Ok(json) => {
            let title = json.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
            let image = json.get("image").and_then(|v| v.as_str()).map(|s| s.to_string());
            let content = json.get("content").and_then(|v| v.as_str()).map(|s| s.to_string());
            (title, image, content)
        }
        Err(e) => {
            log::warn!("Failed to parse memo JSON: {}", e);
            (None, None, None)
        }
    }
}

#[component]
pub fn HomePage(
    session: RwSignal<Session>,
) -> impl IntoView {
    // state for burn records
    let (all_burn_records, set_all_burn_records) = create_signal(Vec::<BurnRecord>::new());
    let (is_loading, set_is_loading) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    let (is_initial_load, set_is_initial_load) = create_signal(true);
    
    // pagination state
    let (current_page, set_current_page) = create_signal(1usize);
    let (total_records, set_total_records) = create_signal(0usize);
    const RECORDS_PER_PAGE: usize = 12;
    
    // pagination helper functions
    let get_total_pages = move || {
        let total = total_records.get();
        (total + RECORDS_PER_PAGE - 1) / RECORDS_PER_PAGE
    };
    
    let get_current_page_records = move || {
        let records = all_burn_records.get();
        let page = current_page.get();
        let start_idx = (page - 1) * RECORDS_PER_PAGE;
        let end_idx = (start_idx + RECORDS_PER_PAGE).min(records.len());
        
        if start_idx < records.len() {
            records[start_idx..end_idx].to_vec()
        } else {
            Vec::new()
        }
    };
    
    let handle_prev_page = move |_| {
        if current_page.get() > 1 {
            set_current_page.update(|p| *p -= 1);
        }
    };
    
    let handle_next_page = move |_| {
        let total_pages = get_total_pages();
        if current_page.get() < total_pages {
            set_current_page.update(|p| *p += 1);
        }
    };
    
    let go_to_page = move |page: usize| {
        let total_pages = get_total_pages();
        if page > 0 && page <= total_pages {
            set_current_page.set(page);
        }
    };

    // silent refresh data (without changing loading state)
    let silent_refresh = move || {
        spawn_local(async move {
            TimeoutFuture::new(50).await;
            
            match get_latest_burn_shard().await {
                Ok(records) => {
                    let total_count = records.len();
                    set_all_burn_records.set(records);
                    set_total_records.set(total_count);
                    set_error_message.set(String::new());
                    
                    if is_initial_load.get_untracked() {
                        set_is_initial_load.set(false);
                    }
                    
                    log::info!("Silently updated {} burn records", total_count);
                }
                Err(e) => {
                    if is_initial_load.get_untracked() {
                        set_error_message.set(format!("Failed to load burn records: {}", e));
                        set_is_initial_load.set(false);
                    }
                    log::error!("Failed to refresh burn records: {}", e);
                }
            }
        });
    };

    // manually refresh data (show loading state)
    let handle_refresh = move |_| {
        set_is_loading.set(true);
        
        spawn_local(async move {
            TimeoutFuture::new(100).await;
            
            match refresh_latest_burn_shard().await {
                Ok(new_records) => {
                    let total_count = new_records.len();
                    set_all_burn_records.set(new_records);
                    set_total_records.set(total_count);
                    set_error_message.set(String::new());
                    set_current_page.set(1);
                    log::info!("Successfully refreshed {} burn records", total_count);
                }
                Err(e) => {
                    set_error_message.set(format!("Failed to refresh burn records: {}. Showing previous data.", e));
                    log::error!("Failed to refresh burn records: {}", e);
                }
            }
            
            set_is_loading.set(false);
        });
    };

    // silent refresh when page loads
    create_effect(move |_| {
        silent_refresh();
    });

    view! {
        <div class="home-page">
            <div class="burn-shard-section">
                <div class="header-section" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                    <h2>
                        "Latest Burns"
                        <Show when=move || { total_records.get() > 0 }>
                            <span style="font-size: 0.8em; color: #666; margin-left: 10px;">
                                {move || format!("({} total records)", total_records.get())}
                            </span>
                        </Show>
                    </h2>
                    <button 
                        class="refresh-btn"
                        on:click=handle_refresh
                        prop:disabled=move || is_loading.get()
                    >
                        {move || if is_loading.get() {
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
                    let error = error_message.get();
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
                    let is_initial = is_initial_load.get();
                    
                    if all_records.is_empty() && is_initial {
                        view! {
                            <div class="loading-container">
                                <div class="loading-spinner"></div>
                                <p class="loading-text">"Loading latest burns..."</p>
                            </div>
                        }
                    } else if all_records.is_empty() && !is_initial {
                        view! {
                            <div class="empty-state">
                                <p class="empty-message">
                                    <i class="fas fa-fire" style="margin-right: 8px;"></i>
                                    "No burn records found"
                                </p>
                            </div>
                        }
                    } else {
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

                                // memo cards - use new BurnRecordWithImage component
                                <div class="memo-cards">
                                    <For
                                        each=move || get_current_page_records()
                                        key=|record| format!("{}_{}", record.blocktime, record.signature)
                                        children=move |record| {
                                            view! {
                                                <BurnRecordWithImage 
                                                    burn_record=record 
                                                    session=session
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
        </div>
    }
} 