use leptos::*;
use crate::core::session::Session;
use crate::core::rpc_base::RpcConnection;
use crate::core::rpc_chat::{ChatStatistics, ChatGroupInfo, ChatMessage, ChatMessagesResponse};
use crate::core::rpc_mint::MintConfig;
use crate::pages::log_view::add_log_entry;
use wasm_bindgen_futures::spawn_local;

// Chat page view mode
#[derive(Clone, PartialEq)]
enum ChatView {
    GroupsList,
    ChatRoom(u64), // group_id
}

#[component]
pub fn ChatPage(session: RwSignal<Session>) -> impl IntoView {
    let (chat_stats, set_chat_stats) = create_signal::<Option<ChatStatistics>>(None);
    let (loading, set_loading) = create_signal(true);
    let (error_message, set_error_message) = create_signal::<Option<String>>(None);
    let (current_view, set_current_view) = create_signal(ChatView::GroupsList);
    
    // Chat room specific states
    let (current_group_info, set_current_group_info) = create_signal::<Option<ChatGroupInfo>>(None);
    let (messages, set_messages) = create_signal::<Vec<ChatMessage>>(vec![]);
    let (message_input, set_message_input) = create_signal(String::new());
    let (sending, set_sending) = create_signal(false);

    // Current mint reward state
    let (current_mint_reward, set_current_mint_reward) = create_signal::<Option<String>>(None);

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

    // Load current mint reward
    spawn_local(async move {
        let rpc = RpcConnection::new();
        match rpc.get_current_mint_reward_formatted().await {
            Ok(reward) => {
                set_current_mint_reward.set(Some(reward));
            },
            Err(e) => {
                log::warn!("Failed to get current mint reward: {}", e);
                // Use default if unable to fetch
                set_current_mint_reward.set(Some("+1.000000 MEMO".to_string()));
            }
        }
    });

    // Function to enter a chat room
    let enter_chat_room = move |group_id: u64| {
        set_current_view.set(ChatView::ChatRoom(group_id));
        
        // Find group info from loaded stats
        if let Some(stats) = chat_stats.get() {
            if let Some(group) = stats.groups.iter().find(|g| g.group_id == group_id) {
                set_current_group_info.set(Some(group.clone()));
            }
        }
        
        // Load messages for this group
        spawn_local(async move {
            set_loading.set(true);
            add_log_entry("INFO", &format!("Loading messages for group {}", group_id));
            
            let rpc = RpcConnection::new();
            match rpc.get_chat_messages(group_id, Some(20), None).await {
                Ok(messages_response) => {
                    add_log_entry("INFO", &format!("Loaded {} messages", messages_response.messages.len()));
                    set_messages.set(messages_response.messages);
                    set_error_message.set(None);
                },
                Err(e) => {
                    let error_msg = format!("Failed to load messages: {}", e);
                    add_log_entry("ERROR", &error_msg);
                    set_error_message.set(Some(error_msg));
                }
            }
            set_loading.set(false);
        });
    };

    // Function to go back to groups list
    let back_to_groups = move |_| {
        set_current_view.set(ChatView::GroupsList);
        set_current_group_info.set(None);
        set_messages.set(vec![]);
        set_message_input.set(String::new());
    };

    // Refresh data function for groups list
    let refresh_groups_data = move |_| {
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

    // Refresh messages function for chat room
    let refresh_messages = move |_| {
        if let ChatView::ChatRoom(group_id) = current_view.get() {
            spawn_local(async move {
                add_log_entry("INFO", "Refreshing messages...");
                
                let rpc = RpcConnection::new();
                match rpc.get_chat_messages(group_id, Some(20), None).await {
                    Ok(messages_response) => {
                        add_log_entry("INFO", &format!("Refreshed {} messages", messages_response.messages.len()));
                        set_messages.set(messages_response.messages);
                        set_error_message.set(None);
                    },
                    Err(e) => {
                        let error_msg = format!("Failed to refresh messages: {}", e);
                        add_log_entry("ERROR", &error_msg);
                        set_error_message.set(Some(error_msg));
                    }
                }
            });
        }
    };

    // Handle message sending (placeholder for now)
    let send_message = move |_ev: web_sys::MouseEvent| {
        let message_text = message_input.get().trim().to_string();
        if message_text.is_empty() {
            return;
        }
        
        set_sending.set(true);
        set_message_input.set(String::new());
        
        spawn_local(async move {
            // TODO: Implement actual message sending when you provide the RPC interface
            add_log_entry("INFO", &format!("TODO: Send message: {}", message_text));
            
            // Simulate sending delay
            gloo_timers::future::TimeoutFuture::new(1000).await;
            
            set_sending.set(false);
            add_log_entry("INFO", "Message sending not yet implemented");
        });
    };

    // Handle Enter key in message input
    let handle_key_press = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            let dummy_event = web_sys::MouseEvent::new("click").unwrap();
            send_message(dummy_event);
        }
    };

    view! {
        <div class="chat-page">
            <Show
                when=move || current_view.get() == ChatView::GroupsList
                fallback=move || {
                    // Chat Room View
                    view! {
                        <div class="chat-room-container">
                            <div class="chat-room-header">
                                <div class="header-left">
                                    <button class="back-button" on:click=back_to_groups>
                                        <i class="fas fa-arrow-left"></i>
                                        "Back to Groups"
                                    </button>
                                </div>
                                
                                <Show
                                    when=move || current_group_info.get().is_some()
                                    fallback=|| view! {
                                        <div class="group-title">
                                            <h1>"Loading Group..."</h1>
                                        </div>
                                    }
                                >
                                    {move || {
                                        current_group_info.get().map(|info| {
                                            view! {
                                                <div class="group-title">
                                                    <h1><i class="fas fa-comments"></i>{info.name}</h1>
                                                    <p class="group-description">{info.description}</p>
                                                </div>
                                            }
                                        })
                                    }}
                                </Show>
                                
                                <div class="header-right">
                                    <button 
                                        class="refresh-button"
                                        on:click=refresh_messages
                                        disabled=move || loading.get()
                                    >
                                        <i class="fas fa-sync-alt"></i>
                                        "Refresh"
                                    </button>
                                </div>
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
                            
                            <div class="chat-container">
                                <div class="messages-area">
                                    <Show
                                        when=move || !loading.get()
                                        fallback=|| view! {
                                            <div class="loading-container">
                                                <div class="loading-spinner"></div>
                                                <p>"Loading messages..."</p>
                                            </div>
                                        }
                                    >
                                        <Show
                                            when=move || !messages.get().is_empty()
                                            fallback=|| view! {
                                                <div class="empty-messages">
                                                    <i class="fas fa-comments-slash"></i>
                                                    <p>"No messages in this group yet"</p>
                                                    <p class="hint">"Be the first to start the conversation!"</p>
                                                </div>
                                            }
                                        >
                                            <div class="messages-list">
                                                <For
                                                    each=move || messages.get()
                                                    key=|message| message.signature.clone()
                                                    children=move |message: ChatMessage| {
                                                        view! { <MessageItem message=message current_mint_reward=current_mint_reward/> }
                                                    }
                                                />
                                            </div>
                                        </Show>
                                    </Show>
                                </div>
                                
                                <div class="message-input-area">
                                    <div class="input-container">
                                        <textarea
                                            class="message-input"
                                            placeholder="Type your message... (Press Enter to send, Shift+Enter for new line)"
                                            prop:value=move || message_input.get()
                                            on:input=move |ev| {
                                                set_message_input.set(event_target_value(&ev));
                                            }
                                            on:keypress=handle_key_press
                                            disabled=move || sending.get()
                                        ></textarea>
                                        <button
                                            class="send-button"
                                            on:click=send_message
                                            disabled=move || message_input.get().trim().is_empty() || sending.get()
                                        >
                                            <Show
                                                when=move || sending.get()
                                                fallback=|| view! { <i class="fas fa-paper-plane"></i> }
                                            >
                                                <div class="spinner"></div>
                                            </Show>
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    }
                }
            >
                // Groups List View (existing code)
                <div class="groups-list-container">
                    <div class="page-header">
                        <h1><i class="fas fa-comments"></i>" Memo Chat"</h1>
                        <p class="page-description">
                            "Chat & Mint"
                        </p>
                        <button 
                            class="refresh-button"
                            on:click=refresh_groups_data
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
                                        <GroupsList groups=stats.groups enter_chat_room=enter_chat_room/>
                                    </div>
                                }
                            })
                        }}
                    </Show>
                </div>
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
fn GroupsList(groups: Vec<ChatGroupInfo>, enter_chat_room: impl Fn(u64) + 'static + Copy) -> impl IntoView {
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
                            view! { <GroupCard group=group enter_chat_room=enter_chat_room/> }
                        }
                    />
                </div>
            </Show>
        </div>
    }
}

#[component]
fn GroupCard(group: ChatGroupInfo, enter_chat_room: impl Fn(u64) + 'static + Copy) -> impl IntoView {
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

    // Handle click to enter chat group
    let handle_click = move |_| {
        enter_chat_room(group_id.get());
    };

    view! {
        <div class="group-card clickable" on:click=handle_click>
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
            
            <div class="enter-chat-hint">
                <i class="fas fa-arrow-right"></i>
                <span>"Click to enter chat group"</span>
            </div>
        </div>
    }
}

#[component]
fn MessageItem(message: ChatMessage, current_mint_reward: ReadSignal<Option<String>>) -> impl IntoView {
    // Store values in variables to make them accessible in closures
    let timestamp = message.timestamp;
    let message_content = message.message.clone();
    let sender = message.sender.clone();
    
    // Helper function to format sender address (first 4 + last 4 chars)
    let format_sender = move |sender: &str| -> String {
        if sender.is_empty() {
            "Anonymous".to_string()
        } else if sender.len() >= 8 {
            format!("{}...{}", &sender[..4], &sender[sender.len()-4..])
        } else {
            sender.to_string()
        }
    };
    
    view! {
        <div class="message-item">
            <div class="message-header">
                <span class="sender">
                    {format_sender(&sender)}
                </span>
                <span class="timestamp">
                    {move || {
                        if timestamp > 0 {
                            format_timestamp(timestamp)
                        } else {
                            "Unknown time".to_string()
                        }
                    }}
                </span>
            </div>
            <div class="message-content">
                {message_content}
            </div>
            <div class="message-meta">
                <i class="fas fa-coins"></i>
                <span class="memo-amount">
                    {move || current_mint_reward.get().unwrap_or_else(|| "+1.000000 MEMO".to_string())}
                </span>
            </div>
        </div>
    }
}

// Helper function to format unix timestamp to readable date
fn format_timestamp(timestamp: i64) -> String {
    log::info!("Formatting timestamp: {}", timestamp);
    
    if timestamp <= 0 {
        log::warn!("Invalid timestamp: {}", timestamp);
        return "Unknown".to_string();
    }
    
    use wasm_bindgen::prelude::*;
    use js_sys::Date;
    
    // Convert unix timestamp to milliseconds for JavaScript Date
    let date = Date::new(&JsValue::from_f64(timestamp as f64 * 1000.0));
    let iso_string = date.to_iso_string();
    
    match iso_string.as_string() {
        Some(iso_str) => {
            log::info!("ISO string: {}", iso_str);
            if iso_str.len() >= 19 {
                let date_part = &iso_str[0..10];
                let time_part = &iso_str[11..16];
                let formatted = format!("{} {}", date_part, time_part);
                log::info!("Formatted time: {}", formatted);
                formatted
            } else {
                let fallback = format!("Timestamp: {}", timestamp);
                log::warn!("Short ISO string, using fallback: {}", fallback);
                fallback
            }
        },
        None => {
            let fallback = format!("Timestamp: {}", timestamp);
            log::warn!("Failed to get ISO string, using fallback: {}", fallback);
            fallback
        }
    }
} 