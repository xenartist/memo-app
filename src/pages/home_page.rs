use leptos::*;
use crate::core::cache::{BurnRecord, get_latest_burn_shard, refresh_latest_burn_shard};
use crate::pages::memo_card::MemoCard;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use gloo_timers::future::sleep;
use std::time::Duration;
use wasm_bindgen::JsCast;

#[component]
pub fn HomePage(
) -> impl IntoView {
    // pagination related signals
    let (all_burn_records, set_all_burn_records) = create_signal(Vec::<BurnRecord>::new());
    let (current_page, set_current_page) = create_signal(1usize);
    let (page_size, _set_page_size) = create_signal(10usize); // show 10 cards per page
    let (total_records, set_total_records) = create_signal(0usize);
    let (is_loading, set_is_loading) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    let (is_initial_load, set_is_initial_load) = create_signal(true);

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

    // format timestamp
    let format_timestamp = |timestamp: i64| -> String {
        let date = js_sys::Date::new(&(timestamp as f64 * 1000.0).into());
        date.to_locale_string("en-US", &js_sys::Object::new()).as_string().unwrap_or_else(|| "Unknown".to_string())
    };

    // silent refresh data (without changing loading state, keeping existing UI)
    let silent_refresh = move || {
        spawn_local(async move {
            // give UI time to render
            TimeoutFuture::new(50).await;
            
            // Use the new cache system instead of session
            match get_latest_burn_shard().await {
                Ok(records) => {
                    // only update display when successfully getting data
                    let total_count = records.len();
                    set_all_burn_records.set(records);
                    set_total_records.set(total_count);
                    set_error_message.set(String::new());
                    
                    // after first load, update state
                    if is_initial_load.get_untracked() {
                        set_is_initial_load.set(false);
                    }
                    
                    log::info!("Silently updated {} burn records", total_count);
                }
                Err(e) => {
                    // only show error on first load, subsequent silent refresh failures do not affect UI
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
            
            // Use the new cache system to force refresh
            match refresh_latest_burn_shard().await {
                Ok(new_records) => {
                    let total_count = new_records.len();
                    set_all_burn_records.set(new_records);
                    set_total_records.set(total_count);
                    set_error_message.set(String::new());
                    // reset to first page when refreshing
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

    // silent refresh when page loads (without affecting existing UI)
    create_effect(move |_| {
        silent_refresh();
    });

    view! {
        <div class="home-page">
            <div class="burn-shard-section">
                <div class="header-section" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                    <h2>
                        "Latest Burns"
                        // show pagination info
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
                        // only show loading when first loading and no data
                        view! {
                            <div class="loading-container">
                                <div class="loading-spinner"></div>
                                <p class="loading-text">"Loading latest burns..."</p>
                            </div>
                        }
                    } else if all_records.is_empty() && !is_initial {
                        // not first load, but no data
                        view! {
                            <div class="empty-state">
                                <p class="empty-message">
                                    <i class="fas fa-fire" style="margin-right: 8px;"></i>
                                    "No burn records found"
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
                                        key=|record| format!("{}_{}", record.blocktime, record.signature)
                                        children=move |record| {
                                            // format pubkey (display first 4 and last 4 characters)
                                            let display_pubkey = if record.pubkey.len() >= 8 {
                                                format!("{}...{}", &record.pubkey[..4], &record.pubkey[record.pubkey.len()-4..])
                                            } else {
                                                record.pubkey.clone()
                                            };
                                            
                                            // format signature (display first 8 and last 8 characters)
                                            let display_signature = if record.signature.len() >= 16 {
                                                format!("{}...{}", &record.signature[..8], &record.signature[record.signature.len()-8..])
                                            } else {
                                                record.signature.clone()
                                            };
                                            
                                            // convert amount to tokens (from lamports)
                                            let amount_tokens = record.amount as f64 / 1_000_000_000.0;
                                            
                                            // generate a placeholder image URL  
                                            let placeholder_image = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNjQiIGhlaWdodD0iNjQiIHZpZXdCb3g9IjAgMCA2NCA2NCIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3Qgd2lkdGg9IjY0IiBoZWlnaHQ9IjY0IiBmaWxsPSIjZjBmMGYwIi8+Cjx0ZXh0IHg9IjMyIiB5PSIzNiIgdGV4dC1hbmNob3I9Im1pZGRsZSIgZm9udC1mYW1pbHk9IkFyaWFsIiBmb250LXNpemU9IjEyIiBmaWxsPSIjNjY2Ij5CdXJuPC90ZXh0Pgo8L3N2Zz4K";
                                            
                                            view! {
                                                <MemoCard
                                                    image=placeholder_image.to_string()
                                                    signature=display_signature
                                                    pubkey=display_pubkey
                                                    blocktime=record.blocktime
                                                    amount=amount_tokens
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
        </div>
    }
} 