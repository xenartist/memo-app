use leptos::*;
use crate::core::session::Session;
use crate::core::rpc_base::RpcConnection;
use crate::core::rpc_chat::{ChatStatistics, ChatGroupInfo};
use crate::pages::log_view::add_log_entry;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn ChatPage(session: RwSignal<Session>) -> impl IntoView {
    let (chat_stats, set_chat_stats) = create_signal::<Option<ChatStatistics>>(None);
    let (loading, set_loading) = create_signal(true);
    let (error_message, set_error_message) = create_signal::<Option<String>>(None);

    // Load chat statistics on component mount
    spawn_local(async move {
        set_loading.set(true);
        set_error_message.set(None);
        
        add_log_entry("INFO", "Loading memo-chat statistics...");
        
        let rpc = RpcConnection::new();
        match rpc.get_all_chat_statistics().await {
            Ok(stats) => {
                add_log_entry("INFO", &format!("Loaded {} chat groups", stats.total_groups));
                set_chat_stats.set(Some(stats));
                set_error_message.set(None);
            },
            Err(e) => {
                let error_msg = format!("Failed to load chat statistics: {}", e);
                add_log_entry("ERROR", &error_msg);
                set_error_message.set(Some(error_msg));
            }
        }
        
        set_loading.set(false);
    });

    // Refresh data function
    let refresh_data = move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error_message.set(None);
            
            add_log_entry("INFO", "Refreshing memo-chat statistics...");
            
            let rpc = RpcConnection::new();
            match rpc.get_all_chat_statistics().await {
                Ok(stats) => {
                    add_log_entry("INFO", &format!("Refreshed {} chat groups", stats.total_groups));
                    set_chat_stats.set(Some(stats));
                    set_error_message.set(None);
                },
                Err(e) => {
                    let error_msg = format!("Failed to refresh chat statistics: {}", e);
                    add_log_entry("ERROR", &error_msg);
                    set_error_message.set(Some(error_msg));
                }
            }
            
            set_loading.set(false);
        });
    };

    view! {
        <div class="chat-page">
            <div class="page-header">
                <h1><i class="fas fa-comments"></i>" Memo Chat"</h1>
                <p class="page-description">
                    "View and interact with memo-chat groups on the blockchain"
                </p>
                <button 
                    class="refresh-button"
                    on:click=refresh_data
                    disabled=move || loading.get()
                >
                    <i class="fas fa-sync-alt"></i>
                    {move || if loading.get() { "Loading..." } else { "Refresh" }}
                </button>
            </div>

            <Show
                when=move || error_message.get().is_some()
                fallback=|| view! { <div></div> }
            >
                <div class="error-message">
                    <i class="fas fa-exclamation-triangle"></i>
                    {move || error_message.get().unwrap_or_default()}
                </div>
            </Show>

            <Show
                when=move || !loading.get() && chat_stats.get().is_some()
                fallback=move || view! {
                    <div class="loading-container">
                        <div class="loading-spinner"></div>
                        <p>"Loading chat groups..."</p>
                    </div>
                }
            >
                {move || {
                    chat_stats.get().map(|stats| {
                        view! {
                            <div class="chat-overview">
                                <OverviewStats stats=stats.clone()/>
                                <GroupsList groups=stats.groups/>
                            </div>
                        }
                    })
                }}
            </Show>
        </div>
    }
}

#[component]
fn OverviewStats(stats: ChatStatistics) -> impl IntoView {
    view! {
        <div class="overview-stats">
            <h2>"Chat Groups Overview"</h2>
            <div class="stats-grid">
                <div class="stat-card">
                    <div class="stat-icon">
                        <i class="fas fa-users"></i>
                    </div>
                    <div class="stat-content">
                        <h3>{stats.total_groups}</h3>
                        <p>"Total Groups"</p>
                    </div>
                </div>
                
                <div class="stat-card">
                    <div class="stat-icon">
                        <i class="fas fa-comments"></i>
                    </div>
                    <div class="stat-content">
                        <h3>{stats.total_memos}</h3>
                        <p>"Total Memos"</p>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn GroupsList(groups: Vec<ChatGroupInfo>) -> impl IntoView {
    // Sort groups by memo count (descending) for display
    let mut sorted_groups = groups;
    sorted_groups.sort_by(|a, b| b.memo_count.cmp(&a.memo_count));
    
    // Create a signal to store the sorted groups
    let (groups_signal, _) = create_signal(sorted_groups);

    view! {
        <div class="groups-list">
            <h2>"Chat Groups"</h2>
            
            <Show
                when=move || !groups_signal.get().is_empty()
                fallback=|| view! {
                    <div class="empty-state">
                        <i class="fas fa-comments-slash"></i>
                        <p>"No chat groups found"</p>
                    </div>
                }
            >
                <div class="groups-grid">
                    <For
                        each=move || groups_signal.get()
                        key=|group| group.group_id
                        children=move |group: ChatGroupInfo| {
                            view! { <GroupCard group=group/> }
                        }
                    />
                </div>
            </Show>
        </div>
    }
}

// Helper function to format unix timestamp to readable date
fn format_timestamp(timestamp: i64) -> String {
    if timestamp <= 0 {
        return "Unknown".to_string();
    }
    
    // Use JavaScript Date for formatting in the browser
    use wasm_bindgen::prelude::*;
    use js_sys::Date;
    
    let date = Date::new(&JsValue::from_f64(timestamp as f64 * 1000.0)); // Convert to milliseconds
    
    // Use toISOString() and format it
    let iso_string = date.to_iso_string();
    match iso_string.as_string() {
        Some(iso_str) => {
            // Extract the date and time part, format as "YYYY-MM-DD HH:MM UTC"
            if iso_str.len() >= 19 {
                let date_part = &iso_str[0..10]; // YYYY-MM-DD
                let time_part = &iso_str[11..16]; // HH:MM
                format!("{} {} UTC", date_part, time_part)
            } else {
                format!("Timestamp: {}", timestamp)
            }
        },
        None => format!("Timestamp: {}", timestamp)
    }
}

#[component]
fn GroupCard(group: ChatGroupInfo) -> impl IntoView {
    // Create signals for the data that will be used in reactive contexts
    let group_name = create_memo(move |_| group.name.clone());
    let group_id = create_memo(move |_| group.group_id);
    let group_image = create_memo(move |_| group.image.clone());
    let group_description = create_memo(move |_| {
        if group.description.len() > 100 {
            format!("{}...", &group.description[..97])
        } else {
            group.description.clone()
        }
    });
    let group_tags = create_memo(move |_| group.tags.clone());
    let group_memo_count = create_memo(move |_| group.memo_count);
    let group_burned_amount = create_memo(move |_| group.burned_amount);
    let group_creator = create_memo(move |_| group.creator.clone());
    let group_min_memo_interval = create_memo(move |_| group.min_memo_interval);

    // Format timestamps using our helper function
    let created_at_formatted = format_timestamp(group.created_at);
    let last_memo_formatted = if group.last_memo_time > 0 {
        format_timestamp(group.last_memo_time)
    } else {
        "No memos yet".to_string()
    };

    view! {
        <div class="group-card">
            <div class="group-header">
                <h3 class="group-name">{move || group_name.get()}</h3>
                <div class="group-id">#{move || group_id.get()}</div>
            </div>
            
            <Show
                when=move || !group_image.get().is_empty()
                fallback=|| view! { <div></div> }
            >
                <div class="group-image">
                    <img src={move || group_image.get()} alt="Group image" loading="lazy"/>
                </div>
            </Show>
            
            <Show
                when=move || !group_description.get().is_empty()
                fallback=|| view! { <div></div> }
            >
                <p class="group-description">{move || group_description.get()}</p>
            </Show>
            
            <div class="group-stats">
                <div class="stat-item">
                    <i class="fas fa-comments"></i>
                    <span>{move || group_memo_count.get()} " memos"</span>
                </div>
                <div class="stat-item">
                    <i class="fas fa-fire"></i>
                    <span>{move || format!("{:.2}", group_burned_amount.get() as f64 / 1_000_000.0)} " MEMO"</span>
                </div>
            </div>
            
            <Show
                when=move || !group_tags.get().is_empty()
                fallback=|| view! { <div></div> }
            >
                <div class="group-tags">
                    <For
                        each=move || group_tags.get()
                        key=|tag| tag.clone()
                        children=move |tag: String| {
                            view! { <span class="tag">{tag}</span> }
                        }
                    />
                </div>
            </Show>
            
            <div class="group-meta">
                <div class="meta-item">
                    <label>"Creator:"</label>
                    <span class="creator-address" title={move || group_creator.get()}>
                        {move || {
                            let creator = group_creator.get();
                            format!("{}...{}", &creator[..4], &creator[creator.len()-4..])
                        }}
                    </span>
                </div>
                <div class="meta-item">
                    <label>"Created:"</label>
                    <span>{created_at_formatted}</span>
                </div>
                <div class="meta-item">
                    <label>"Last memo:"</label>
                    <span>{last_memo_formatted}</span>
                </div>
                {
                    if group.min_memo_interval > 0 {
                        view! {
                            <div class="meta-item">
                                <label>"Min interval:"</label>
                                <span>{move || group_min_memo_interval.get()} "s"</span>
                            </div>
                        }
                    } else {
                        view! { <div style="display: none;"></div> }
                    }
                }
            </div>
        </div>
    }
} 