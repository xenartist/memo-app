use leptos::*;
use leptos::html::Div;
use wasm_bindgen::JsCast;
use crate::core::session::Session;
use crate::core::rpc_base::RpcConnection;
use crate::core::rpc_chat::{ChatStatistics, ChatGroupInfo, ChatMessage, ChatMessagesResponse, LocalChatMessage, MessageStatus};
use crate::core::rpc_mint::MintConfig;
use crate::pages::log_view::add_log_entry;
use crate::pages::memo_card::LazyPixelView;
use crate::pages::pixel_view::PixelView;
use crate::core::pixel::Pixel;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use web_sys::{HtmlInputElement, File, FileReader, Event, ProgressEvent, window};
use wasm_bindgen::{closure::Closure};
use js_sys::Uint8Array;
use std::rc::Rc;

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
    let (messages, set_messages) = create_signal::<Vec<LocalChatMessage>>(vec![]);
    let (message_input, set_message_input) = create_signal(String::new());
    let (sending, set_sending) = create_signal(false);

    // Current mint reward state
    let (current_mint_reward, set_current_mint_reward) = create_signal::<Option<String>>(None);
    
    // Node ref for messages area to enable auto-scroll
    let messages_area_ref = create_node_ref::<Div>();
    
    // Create Chat Group Dialog states
    let (show_create_dialog, set_show_create_dialog) = create_signal(false);
    
    // Auto-scroll to bottom when messages change
    create_effect(move |_| {
        let _ = messages.get(); // Track messages changes
        
        // Small delay to ensure DOM is updated
        spawn_local(async move {
            TimeoutFuture::new(50).await;
            
            if let Some(messages_area) = messages_area_ref.get() {
                messages_area.set_scroll_top(messages_area.scroll_height());
            }
        });
    });

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
                set_current_mint_reward.set(Some("+1 MEMO".to_string()));
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
                    // Convert chain messages to local messages
                    let local_messages: Vec<LocalChatMessage> = messages_response.messages
                        .into_iter()
                        .map(LocalChatMessage::from_chain_message)
                        .collect();
                    set_messages.set(local_messages);
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
                        // Convert chain messages to local messages, preserving any local pending messages
                        let current_messages = messages.get();
                        let mut new_local_messages: Vec<LocalChatMessage> = messages_response.messages
                            .into_iter()
                            .map(LocalChatMessage::from_chain_message)
                            .collect();
                        
                        // Add any local pending messages that are not yet on chain
                        for local_msg in current_messages {
                            if local_msg.is_local && local_msg.status == MessageStatus::Sending {
                                // Check if this local message appears in chain messages by comparing content and sender
                                let found_on_chain = new_local_messages.iter().any(|chain_msg| {
                                    chain_msg.message.message == local_msg.message.message &&
                                    chain_msg.message.sender == local_msg.message.sender
                                });
                                
                                if found_on_chain {
                                    // Update local message to sent status
                                    if let Some(chain_msg) = new_local_messages.iter_mut().find(|chain_msg| {
                                        chain_msg.message.message == local_msg.message.message &&
                                        chain_msg.message.sender == local_msg.message.sender
                                    }) {
                                        chain_msg.status = MessageStatus::Sent;
                                    }
                                } else {
                                    // Keep the local pending message
                                    new_local_messages.push(local_msg);
                                }
                            }
                        }
                        
                        // Sort by timestamp
                        new_local_messages.sort_by(|a, b| a.message.timestamp.cmp(&b.message.timestamp));
                        
                        set_messages.set(new_local_messages);
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

    // Handle message sending
    let send_message = move |_ev: web_sys::MouseEvent| {
        let message_text = message_input.get().trim().to_string();
        if message_text.is_empty() {
            return;
        }
        
        // Get current group ID and user info
        if let ChatView::ChatRoom(group_id) = current_view.get() {
            if let Ok(user_pubkey) = session.with_untracked(|s| s.get_public_key()) {
                // Check SOL balance before sending
                let sol_balance = session.with_untracked(|s| s.get_sol_balance());
                if sol_balance < 0.01 {
                    let error_msg = format!("Balance insufficient! Current XNT balance: {:.4}, sending message requires at least 0.01 SOL as transaction fee. Please top up.", sol_balance);
                    add_log_entry("ERROR", &error_msg);
                    set_error_message.set(Some(error_msg));
                    return;
                }
                
                // Clear any previous error messages
                set_error_message.set(None);
                
                // 1. show message on UI immediately
                let local_message = LocalChatMessage::new_local(
                    user_pubkey.clone(),
                    message_text.clone(),
                    group_id
                );
                
                // add to current message list
                set_messages.update(|msgs| {
                    msgs.push(local_message.clone());
                });
                
                // clear input and set sending state
                set_message_input.set(String::new());
                set_sending.set(true);
                
                // 2. short delay to update UI
                spawn_local(async move {
                    TimeoutFuture::new(100).await;
                    
                    // 3. actually send message
                    let result = session.with_untracked(|s| s.clone()).send_chat_message_with_timeout(
                        group_id,
                        &message_text,
                        None, // receiver
                        None, // reply_to_sig
                        Some(30000) // timeout_ms: 30 seconds timeout
                    ).await;
                    
                    log::info!("Chat page: Received result from session: success={}", result.is_ok());
                    
                    match result {
                        Ok(signature) => {
                            add_log_entry("INFO", &format!("Message sent successfully! Signature: {}", signature));
                            
                            // 4. update local message status to sent
                            set_messages.update(|msgs| {
                                if let Some(msg) = msgs.iter_mut().find(|m| {
                                    m.is_local && 
                                    m.message.message == message_text && 
                                    m.message.sender == user_pubkey
                                }) {
                                    msg.status = MessageStatus::Sent;
                                    msg.message.signature = signature; // update to real signature
                                }
                            });
                            
                            // 5. update session balance - directly update balance instead of just marking update needed
                            spawn_local(async move {
                                let mut session_update = session.get_untracked();
                                match session_update.fetch_and_update_balances().await {
                                    Ok(()) => {
                                        log::info!("Successfully updated balances after sending message");
                                        // set balance to original session
                                        session.update(|s| {
                                            s.set_balances(session_update.get_sol_balance(), session_update.get_token_balance());
                                        });
                                    },
                                    Err(e) => {
                                        log::error!("Failed to update balances after sending message: {}", e);
                                        // if direct update fails, revert to marking update needed
                                        session.update(|s| {
                                            s.mark_balance_update_needed();
                                        });
                                    }
                                }
                            });
                            
                            add_log_entry("INFO", "Message status updated to Sent");
                        },
                        Err(e) => {
                            log::error!("Chat page: Error received from session: {}", e);
                            
                            // Parse error to extract specific error message
                            let error_string = e.to_string();
                            let user_friendly_error = 
                                // Try to extract specific error message after " - "
                                if let Some(dash_pos) = error_string.rfind(" - ") {
                                    let specific_msg = &error_string[dash_pos + 3..];
                                    // Clean up the message (remove trailing dots if any)
                                    let cleaned_msg = specific_msg.trim_end_matches('.');
                                    if !cleaned_msg.is_empty() {
                                        cleaned_msg.to_string()
                                    } else {
                                        // Fallback to checking known error types
                                        if error_string.contains("MemoTooFrequent") || error_string.contains("6009") {
                                            "Message sent too frequently. Please wait before sending another message.".to_string()
                                        } else if error_string.contains("timeout") {
                                            "Message send timeout. Please try again.".to_string()
                                        } else if error_string.contains("insufficient") {
                                            "Insufficient balance".to_string()
                                        } else {
                                            "Failed to send message. Please try again.".to_string()
                                        }
                                    }
                                } else {
                                    // Fallback to checking known error types
                                    if error_string.contains("MemoTooFrequent") || error_string.contains("6009") {
                                        "Message sent too frequently. Please wait before sending another message.".to_string()
                                    } else if error_string.contains("timeout") {
                                        "Message send timeout. Please try again.".to_string()
                                    } else if error_string.contains("insufficient") {
                                        "Insufficient balance".to_string()
                                    } else {
                                        "Failed to send message. Please try again.".to_string()
                                    }
                                };
                            
                            add_log_entry("ERROR", &format!("Failed to send message: {}", user_friendly_error));
                            set_error_message.set(Some(user_friendly_error.to_string()));
                            
                            // 6. update local message status to failed
                            set_messages.update(|msgs| {
                                let found = msgs.iter_mut().find(|m| {
                                    m.is_local && 
                                    m.message.message == message_text && 
                                    m.message.sender == user_pubkey
                                });
                                
                                if let Some(msg) = found {
                                    log::info!("Updating message status to Failed");
                                    msg.status = MessageStatus::Failed;
                                } else {
                                    log::error!("Could not find message to update status");
                                }
                            });
                        }
                    }
                    
                    set_sending.set(false);
                });
            } else {
                add_log_entry("ERROR", "Failed to get user public key");
            }
        } else {
            add_log_entry("ERROR", "No chat room selected");
        }
    };

    // Handle retry sending a failed message
    let retry_message = move |message_content: String| {
        // Get current group ID and user info
        if let ChatView::ChatRoom(group_id) = current_view.get() {
            if let Ok(user_pubkey) = session.with_untracked(|s| s.get_public_key()) {
                // Check SOL balance before sending
                let sol_balance = session.with_untracked(|s| s.get_sol_balance());
                if sol_balance < 0.01 {
                    let error_msg = format!("Balance insufficient! Current XNT balance: {:.4}, sending message requires at least 0.01 SOL as transaction fee. Please top up.", sol_balance);
                    add_log_entry("ERROR", &error_msg);
                    set_error_message.set(Some(error_msg));
                    return;
                }
                
                // Clear any previous error messages
                set_error_message.set(None);
                
                // 1. Update the failed message back to sending status
                set_messages.update(|msgs| {
                    if let Some(msg) = msgs.iter_mut().find(|m| {
                        m.is_local && 
                        m.message.message == message_content && 
                        m.message.sender == user_pubkey &&
                        (m.status == MessageStatus::Failed || m.status == MessageStatus::Timeout)
                    }) {
                        log::info!("Updating message status from {:?} to Sending for retry", msg.status);
                        msg.status = MessageStatus::Sending;
                    }
                });
                
                set_sending.set(true);
                
                // 2. short delay to update UI
                spawn_local(async move {
                    TimeoutFuture::new(100).await;
                    
                    // 3. actually send message (retry logic)
                    let result = session.with_untracked(|s| s.clone()).send_chat_message_with_timeout(
                        group_id,
                        &message_content,
                        None, // receiver
                        None, // reply_to_sig
                        Some(30000) // timeout_ms: 30 seconds timeout
                    ).await;
                    
                    log::info!("Retry result: success={}", result.is_ok());
                    
                    match result {
                        Ok(signature) => {
                            add_log_entry("INFO", &format!("Message retry sent successfully! Signature: {}", signature));
                            
                            // 4. update local message status to sent
                            set_messages.update(|msgs| {
                                if let Some(msg) = msgs.iter_mut().find(|m| {
                                    m.is_local && 
                                    m.message.message == message_content && 
                                    m.message.sender == user_pubkey
                                }) {
                                    msg.status = MessageStatus::Sent;
                                    msg.message.signature = signature; // update to real signature
                                }
                            });
                            
                            // 5. update session balance - directly update balance instead of just marking update needed
                            spawn_local(async move {
                                let mut session_update = session.get_untracked();
                                match session_update.fetch_and_update_balances().await {
                                    Ok(()) => {
                                        log::info!("Successfully updated balances after retry sending message");
                                        // set balance to original session
                                        session.update(|s| {
                                            s.set_balances(session_update.get_sol_balance(), session_update.get_token_balance());
                                        });
                                    },
                                    Err(e) => {
                                        log::error!("Failed to update balances after retry sending message: {}", e);
                                        // if direct update fails, revert to marking update needed
                                        session.update(|s| {
                                            s.mark_balance_update_needed();
                                        });
                                    }
                                }
                            });
                            
                            add_log_entry("INFO", "Retry message status updated to Sent");
                        },
                        Err(e) => {
                            log::error!("Retry failed: {}", e);
                            
                            // Parse error to show user-friendly English message
                            let user_friendly_error = if e.to_string().contains("MemoTooFrequent") || e.to_string().contains("6009") {
                                "Message sent too frequently. Please wait before sending another message."
                            } else if e.to_string().contains("timeout") {
                                "Message send timeout. Please try again."
                            } else if e.to_string().contains("insufficient") {
                                "Insufficient balance"
                            } else {
                                "Failed to send message. Please try again."
                            };
                            
                            add_log_entry("ERROR", &format!("Retry failed: {}", user_friendly_error));
                            set_error_message.set(Some(user_friendly_error.to_string()));
                            
                            // 6. update local message status back to failed
                            set_messages.update(|msgs| {
                                if let Some(msg) = msgs.iter_mut().find(|m| {
                                    m.is_local && 
                                    m.message.message == message_content && 
                                    m.message.sender == user_pubkey
                                }) {
                                    msg.status = MessageStatus::Failed;
                                }
                            });
                        }
                    }
                    
                    set_sending.set(false);
                });
            } else {
                add_log_entry("ERROR", "Failed to get user public key for retry");
            }
        } else {
            add_log_entry("ERROR", "No chat room selected for retry");
        }
    };

    // Handle Enter key in message input
    let handle_key_press = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            let dummy_event = web_sys::MouseEvent::new("click").unwrap();
            send_message(dummy_event);
        }
    };

    // Helper function to extract fallback error messages
    let extract_fallback_error_message = |error_str: &str| -> String {
        if error_str.contains("MemoTooFrequent") || error_str.contains("6009") {
            "Message sent too frequently. Please wait before sending another message.".to_string()
        } else if error_str.contains("timeout") {
            "Message send timeout. Please try again.".to_string()
        } else if error_str.contains("insufficient") {
            "Insufficient balance".to_string()
        } else {
            "Failed to send message. Please try again.".to_string()
        }
    };

    // Function to open create chat group dialog
    let open_create_dialog = move |_| {
        set_show_create_dialog.set(true);
    };

    // Function to close create chat group dialog
    let close_create_dialog = move || {
        set_show_create_dialog.set(false);
    };

    // Function to handle successful group creation
    let on_group_created = move |signature: String, group_id: u64| {
        add_log_entry("INFO", &format!("Chat group created successfully! ID: {}, Signature: {}", group_id, signature));
        set_show_create_dialog.set(false);
        
        // Wait 30 seconds before refreshing to allow blockchain to update
        spawn_local(async move {
            add_log_entry("INFO", "Waiting 30 seconds for blockchain to update...");
            TimeoutFuture::new(30_000).await; // Wait 30 seconds
            
            add_log_entry("INFO", "Refreshing group list after group creation...");
            refresh_groups_data(web_sys::MouseEvent::new("click").unwrap());
        });
    };

    // Function to handle group creation error
    let on_group_creation_error = move |error: String| {
        add_log_entry("ERROR", &format!("Failed to create chat group: {}", error));
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
                                <div class="messages-area" node_ref=messages_area_ref>
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
                                                    key=|message| format!("{}_{:?}", message.message.signature, message.status)
                                                    children=move |message: LocalChatMessage| {
                                                        view! { <MessageItem message=message current_mint_reward=current_mint_reward session=session retry_callback=retry_message/> }
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
                                            placeholder=move || {
                                                if sending.get() {
                                                    "Sending, please wait...".to_string()
                                                } else if session.with(|s| s.get_sol_balance()) < 0.005 {
                                                    format!("Balance insufficient, sending message requires at least 0.005 XNT (current: {:.4} XNT)", session.with(|s| s.get_sol_balance()))
                                                } else {
                                                    "Type your message... (Press Enter to send, Shift+Enter for new line)".to_string()
                                                }
                                            }
                                            prop:value=move || message_input.get()
                                            on:input=move |ev| {
                                                set_message_input.set(event_target_value(&ev));
                                            }
                                            on:keypress=handle_key_press
                                            disabled=move || sending.get() || session.with(|s| s.get_sol_balance()) < 0.005
                                        ></textarea>
                                        <button
                                            class="send-button"
                                            on:click=send_message
                                            disabled=move || {
                                                message_input.get().trim().is_empty() || 
                                                sending.get() || 
                                                session.with(|s| s.get_sol_balance()) < 0.005
                                            }
                                            title=move || {
                                                if sending.get() {
                                                    "Sending...".to_string()
                                                } else if session.with(|s| s.get_sol_balance()) < 0.005 {
                                                    format!("Balance insufficient, sending message requires at least 0.005 XNT (current: {:.4} XNT)", session.with(|s| s.get_sol_balance()))
                                                } else if message_input.get().trim().is_empty() {
                                                    "Please enter message content".to_string()
                                                } else {
                                                    "Send message".to_string()
                                                }
                                            }
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
                        <h1><i class="fas fa-comments"></i>"Chat & Earn"</h1>
                        <p class="page-description">
                            "Decentralized Messaging on X1 Blockchain"
                        </p>
                        <div class="header-buttons">
                            <button 
                                class="create-group-button"
                                on:click=open_create_dialog
                                disabled=move || loading.get()
                                title=move || {
                                    if !session.with(|s| s.has_user_profile()) {
                                        "Please create your profile first".to_string()
                                    } else {
                                        "Create new chat group".to_string()
                                    }
                                }
                            >
                                <i class="fas fa-plus"></i>
                                "Create Group"
                            </button>
                            <button 
                                class="refresh-button"
                                on:click=refresh_groups_data
                                disabled=move || loading.get()
                            >
                                <i class="fas fa-sync-alt"></i>
                                {move || if loading.get() { "Loading..." } else { "Refresh" }}
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

            // Create Chat Group Dialog
            <Show when=move || show_create_dialog.get()>
                <div class="modal-overlay">
                    <CreateChatGroupForm
                        session=session
                        on_close=Rc::new(close_create_dialog)
                        on_success=Rc::new(on_group_created)
                        on_error=Rc::new(on_group_creation_error)
                    />
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
                        <p>"Total Messages"</p>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn GroupsList(groups: Vec<ChatGroupInfo>, enter_chat_room: impl Fn(u64) + 'static + Copy) -> impl IntoView {
    // Sort groups by burned amount (descending) for display
    let mut sorted_groups = groups;
    sorted_groups.sort_by(|a, b| b.burned_amount.cmp(&a.burned_amount));
    
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
        "No messages yet".to_string()
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
                when=move || true // always show image area
                fallback=|| view! { <div></div> }
            >
                <div class="group-image">
                    {move || {
                        let image_data = group_image.get();
                        
                        // check if it is a valid pixel art string (starts with "c:" or "n:")
                        if !image_data.is_empty() && 
                           (image_data.starts_with("c:") || image_data.starts_with("n:")) {
                            // valid pixel art string
                            view! {
                                <LazyPixelView
                                    art={image_data}
                                    size=64
                                />
                            }.into_view()
                        } else if !image_data.is_empty() && 
                                  (image_data.starts_with("http") || image_data.starts_with("data:")) {
                            // regular image URL
                            view! {
                                <img 
                                    src={image_data}
                                    alt="Group image" 
                                    class="group-image-img"
                                    loading="lazy"
                                />
                            }.into_view()
                        } else {
                            // no valid image, generate random pixel art based on group_id
                            let group_id_val = group_id.get();
                            let fake_pixel_art = generate_random_pixel_art(group_id_val);
                            
                            view! {
                                <LazyPixelView
                                    art={fake_pixel_art}
                                    size=64
                                />
                            }.into_view()
                        }
                    }}
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
                    <span>{move || group_memo_count.get()} " messages"</span>
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
                    <label>"Last message:"</label>
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
fn MessageItem(
    message: LocalChatMessage, 
    current_mint_reward: ReadSignal<Option<String>>, 
    session: RwSignal<Session>,
    retry_callback: impl Fn(String) + 'static + Copy
) -> impl IntoView {
    // Store values in variables to make them accessible in closures
    let timestamp = message.message.timestamp;
    let message_content = message.message.message.clone();
    let sender = message.message.sender.clone();
    let status = message.status;
    let is_local = message.is_local;
    let message_type = message.message.message_type.clone();
    let burn_amount = message.message.burn_amount;
    
    // Create clones for different uses to avoid move issues
    let message_type_for_class = message_type.clone();
    let message_type_for_display = message_type.clone();
    let message_type_for_meta = message_type.clone();
    
    // Check if this message is from the current user
    let is_current_user = session.with_untracked(|s| {
        if let Ok(current_pubkey) = s.get_public_key() {
            current_pubkey == sender
        } else {
            false
        }
    });
    
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
        <div 
            class="message-item" 
            class:message-sending=move || status == MessageStatus::Sending
            class:message-current-user=move || is_current_user
            class:message-burn=move || message_type_for_class == "burn"
        >
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
            <div class="message-content-wrapper">
                <div class="message-content">
                    {message_content.clone()}
                </div>
                // show status for local messages
                {
                    let message_content = message_content.clone(); // Clone for closure
                    move || {
                        if is_local {
                            view! {
                                <div class="message-status-corner">
                                    {
                                        match status {
                                            MessageStatus::Sending => view! {
                                                <span class="status-sending">
                                                    <i class="fas fa-clock"></i>
                                                    "Sending..."
                                                </span>
                                            }.into_view(),
                                            MessageStatus::Sent => view! {
                                                <span class="status-sent">
                                                    <i class="fas fa-check"></i>
                                                    "Sent"
                                                </span>
                                            }.into_view(),
                                            MessageStatus::Failed => view! {
                                                <span class="status-failed">
                                                    <i class="fas fa-exclamation-triangle"></i>
                                                    "Failed"
                                                    <button 
                                                        class="retry-button"
                                                        on:click={
                                                            let message_content = message_content.clone();
                                                            move |_| {
                                                                log::info!("Retry sending message: {}", message_content);
                                                                retry_callback(message_content.clone());
                                                            }
                                                        }
                                                        title="Retry sending this message"
                                                    >
                                                        <i class="fas fa-redo"></i>
                                                        "Retry"
                                                    </button>
                                                </span>
                                            }.into_view(),
                                            MessageStatus::Timeout => view! {
                                                <span class="status-timeout">
                                                    <i class="fas fa-clock"></i>
                                                    "Timeout"
                                                    <button 
                                                        class="retry-button"
                                                        on:click={
                                                            let message_content = message_content.clone();
                                                            move |_| {
                                                                log::info!("Retry sending message: {}", message_content);
                                                                retry_callback(message_content.clone());
                                                            }
                                                        }
                                                        title="Retry sending this message"
                                                    >
                                                        <i class="fas fa-redo"></i>
                                                        "Retry"
                                                    </button>
                                                </span>
                                            }.into_view(),
                                            _ => view! { <div></div> }.into_view(),
                                        }
                                    }
                                </div>
                            }.into_view()
                        } else {
                            view! { <div></div> }.into_view()
                        }
                    }
                }
            </div>
            <div class="message-meta">
                {
                    if message_type_for_meta == "burn" {
                        view! {
                            <div class="burn-amount">
                                <i class="fas fa-fire"></i>
                                <span>
                                    {move || {
                                        if let Some(amount) = burn_amount {
                                            format!("Burn {:.2} MEMO", amount as f64 / 1_000_000.0)
                                        } else {
                                            "Burn operation".to_string()
                                        }
                                    }}
                                </span>
                            </div>
                        }.into_view()
                    } else {
                        view! {
                            <div class="memo-amount">
                                <i class="fas fa-coins"></i>
                                <span>
                                    {move || current_mint_reward.get().unwrap_or_else(|| "+1 MEMO".to_string())}
                                </span>
                            </div>
                        }.into_view()
                    }
                }
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

// generate random pixel art string (simplest random fill)
fn generate_random_pixel_art(seed: u64) -> String {
    // add debug log
    log::info!("Generating pixel art with seed: {}", seed);
    
    // create 16x16 pixel art
    let mut pixel = Pixel::new_with_size(16);
    
    // ensure seed is not 0, avoid xorshift stuck in all zeros
    let mut rng_state = if seed == 0 { 1 } else { seed };
    
    // fill random pixel data
    for y in 0..16 {
        for x in 0..16 {
            // use xorshift algorithm, better randomness
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 7;
            rng_state ^= rng_state << 17;
            
            let is_black = (rng_state % 100) < 40; // 40% probability of black
            pixel.set(x, y, is_black);
        }
    }
    
    let result = pixel.to_optimal_string();
    log::info!("Generated pixel art for seed {}: length={}, preview={}", 
        seed, result.len(), 
        if result.len() > 30 { &result[..30] } else { &result }
    );
    result
} 

#[component]
fn CreateChatGroupForm(
    session: RwSignal<Session>,
    on_close: Rc<dyn Fn()>,
    on_success: Rc<dyn Fn(String, u64)>,
    on_error: Rc<dyn Fn(String)>,
) -> impl IntoView {
    // Wrap callbacks in signals for easy access in closures
    let on_close_signal = create_rw_signal(Some(on_close));
    let on_success_signal = create_rw_signal(Some(on_success));
    let on_error_signal = create_rw_signal(Some(on_error));

    // Form state signals
    let (group_name, set_group_name) = create_signal(String::new());
    let (group_description, set_group_description) = create_signal(String::new());
    let (group_tags, set_group_tags) = create_signal(String::new()); // comma-separated tags
    let (min_memo_interval, set_min_memo_interval) = create_signal(60i64); // default 60 seconds
    let (burn_amount, set_burn_amount) = create_signal(42069u64); // default 42,069 tokens (minimum required)
    let (pixel_art, set_pixel_art) = create_signal(Pixel::new_with_size(16)); // default 16x16
    
    // UI state signals
    let (is_creating, set_is_creating) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    let (show_copied, set_show_copied) = create_signal(false);
    let (creating_status, set_creating_status) = create_signal(String::new());

    // Grid size for pixel art
    let (grid_size, set_grid_size) = create_signal(16usize);

    // Create combined image data
    let get_image_data = move || -> String {
        pixel_art.get().to_optimal_string()
    };

    // Calculate current memo size in bytes (Borsh + Base64)
    let calculate_memo_size = move || -> (usize, bool, String) {
        let name = group_name.get().trim().to_string();
        let description = group_description.get().trim().to_string();
        let image_data = get_image_data();
        // Parse tags inline here
        let tags = group_tags.get()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .take(4) // Maximum 4 tags
            .collect();
        let interval = Some(min_memo_interval.get());
        let amount = burn_amount.get() * 1_000_000; // Convert to lamports
        
        // Create temporary ChatGroupCreationData for size calculation
        let group_data = crate::core::rpc_chat::ChatGroupCreationData::new(
            0, // temporary group_id
            name,
            description,
            image_data,
            tags,
            interval,
        );
        
        match group_data.calculate_final_memo_size(amount) {
            Ok(size) => {
                let is_valid = size >= 69 && size <= 800;
                let status = if is_valid {
                    "✅ Valid".to_string()
                } else if size < 69 {
                    "❌ Too short".to_string()
                } else {
                    "❌ Too long".to_string()
                };
                (size, is_valid, status)
            },
            Err(e) => (0, false, format!("❌ Error: {}", e))
        }
    };

    // Parse tags from comma-separated string
    let parse_tags = move || -> Vec<String> {
        group_tags.get()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .take(4) // Maximum 4 tags
            .collect()
    };

    // Handle form submission
    let handle_submit = move |ev: leptos::leptos_dom::ev::SubmitEvent| {
        ev.prevent_default();

        if is_creating.get() {
            return;
        }

        // Validate form
        let name = group_name.get().trim().to_string();
        let description = group_description.get().trim().to_string();
        let tags = parse_tags();
        let interval = min_memo_interval.get();
        let amount = burn_amount.get();

        // Validation
        if name.is_empty() || name.len() > 64 {
            set_error_message.set("❌ Group name must be 1-64 characters, got {}".to_string().replace("{}", &name.len().to_string()));
            return;
        }
        if description.len() > 128 {
            set_error_message.set("❌ Group description must be at most 128 characters, got {}".to_string().replace("{}", &description.len().to_string()));
            return;
        }
        if amount < 42069 {
            set_error_message.set("❌ Burn amount must be at least 42,069 MEMO tokens".to_string());
            return;
        }
        if tags.len() > 4 {
            set_error_message.set("❌ Maximum 4 tags allowed".to_string());
            return;
        }
        for tag in &tags {
            if tag.len() > 32 {
                set_error_message.set("❌ Each tag must be at most 32 characters".to_string());
                return;
            }
        }
        if interval < 0 || interval > 86400 {
            set_error_message.set("❌ Memo interval must be between 0 and 86400 seconds (24 hours)".to_string());
            return;
        }

        // Check balance
        let token_balance = session.with_untracked(|s| s.get_token_balance());
        if token_balance < amount as f64 {
            set_error_message.set(format!("❌ Insufficient balance. Required: {} MEMO, Available: {:.2} MEMO", amount, token_balance));
            return;
        }

        // Set UI state
        set_is_creating.set(true);
        set_creating_status.set("Creating chat group...".to_string());
        set_error_message.set(String::new());

        // Create chat group
        spawn_local(async move {
            // Give UI time to update the loading state
            TimeoutFuture::new(100).await;
            
            let mut session_update = session.get_untracked();
            let result = session_update.create_chat_group(
                &name,
                &description,
                &get_image_data(),
                tags,
                Some(interval),
                amount * 1_000_000, // Convert to lamports
            ).await;

            set_is_creating.set(false);
            set_creating_status.set(String::new());

            match result {
                Ok((signature, group_id)) => {
                    // Update session to trigger balance refresh
                    session.update(|s| {
                        s.mark_balance_update_needed();
                    });

                    on_success_signal.with_untracked(|cb_opt| {
                        if let Some(callback) = cb_opt.as_ref() {
                            callback(signature, group_id);
                        }
                    });
                },
                Err(e) => {
                    let error_msg = format!("Failed to create chat group: {}", e);
                    set_error_message.set(format!("❌ {}", error_msg));
                    
                    on_error_signal.with_untracked(|cb_opt| {
                        if let Some(callback) = cb_opt.as_ref() {
                            callback(error_msg);
                        }
                    });
                }
            }
        });
    };

    // Handle image import (similar to mint_form.rs)
    let handle_import = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let input: HtmlInputElement = document
            .create_element("input")
            .unwrap()
            .dyn_into()
            .unwrap();
        
        input.set_type("file");
        input.set_accept("image/*");
        
        let pixel_art_write = set_pixel_art;
        let error_signal = set_error_message;
        let grid_size_signal = grid_size;
        
        let onchange = Closure::wrap(Box::new(move |event: Event| {
            let input: HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
            if let Some(file) = input.files().unwrap().get(0) {
                let reader = FileReader::new().unwrap();
                let reader_clone = reader.clone();
                let current_grid_size = grid_size_signal.get(); // get current size
                
                let onload = Closure::wrap(Box::new(move |_: ProgressEvent| {
                    if let Ok(buffer) = reader_clone.result() {
                        let array = Uint8Array::new(&buffer);
                        let data = array.to_vec();
                        
                        match Pixel::from_image_data_with_size(&data, current_grid_size) {
                            Ok(new_art) => {
                                pixel_art_write.set(new_art);
                                error_signal.set(String::new());
                            }
                            Err(e) => {
                                error_signal.set(format!("Failed to process image: {}", e));
                            }
                        }
                    }
                }) as Box<dyn FnMut(ProgressEvent)>);
                
                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                
                reader.read_as_array_buffer(&file).unwrap();
            }
        }) as Box<dyn FnMut(_)>);
        
        input.set_onchange(Some(onchange.as_ref().unchecked_ref()));
        onchange.forget();
        
        input.click();
    };

    // Handle copy pixel art string
    let copy_string = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        
        let art_string = pixel_art.get().to_optimal_string();
        if let Some(window) = window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(&art_string);
            set_show_copied.set(true);
            
            spawn_local(async move {
                TimeoutFuture::new(3000).await;
                set_show_copied.set(false);
            });
        }
    };

    // Handle close
    let handle_close = move |_| {
        on_close_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };

    view! {
        <div class="create-chat-group-form">
            // Header with title and close button
            <div class="form-header">
                <h3 class="form-title">
                    <i class="fas fa-users"></i>
                    "Create New Chat Group"
                </h3>
                <button
                    type="button"
                    class="form-close-btn"
                    on:click=handle_close
                    title="Close"
                >
                    <i class="fas fa-times"></i>
                </button>
            </div>
            
            <form class="chat-group-form" on:submit=handle_submit>
                <div class="form-layout">
                    // Left side: Basic Information
                    <div class="form-left">
                        // Group Name
                        <div class="form-group">
                            <label for="group-name">
                                <i class="fas fa-pencil-alt"></i>
                                "Group Name (required) *"
                            </label>
                            <input
                                type="text"
                                id="group-name"
                                prop:value=group_name
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_group_name.set(value);
                                }
                                placeholder="Enter group name (1-64 characters)..."
                                maxlength="64"
                                prop:disabled=move || is_creating.get()
                                required
                            />
                        </div>

                        // Group Description
                        <div class="form-group">
                            <label for="group-description">
                                <i class="fas fa-align-left"></i>
                                "Group Description (optional)"
                            </label>
                            <textarea
                                id="group-description"
                                prop:value=group_description
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_group_description.set(value);
                                }
                                placeholder="Enter group description (max 128 characters)..."
                                maxlength="128"
                                rows="3"
                                prop:disabled=move || is_creating.get()
                            ></textarea>
                        </div>

                        // Tags
                        <div class="form-group">
                            <label for="group-tags">
                                <i class="fas fa-tags"></i>
                                "Tags (optional)"
                            </label>
                            <input
                                type="text"
                                id="group-tags"
                                prop:value=group_tags
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_group_tags.set(value);
                                }
                                placeholder="Enter tags separated by commas (max 4 tags, 32 chars each)..."
                                prop:disabled=move || is_creating.get()
                            />
                            <small class="form-hint">
                                <i class="fas fa-info-circle"></i>
                                "Example: technology, blockchain, discussion"
                            </small>
                        </div>

                        // Min Memo Interval
                        <div class="form-group">
                            <label for="memo-interval">
                                <i class="fas fa-clock"></i>
                                "Minimum Message Interval (seconds)"
                            </label>
                            <input
                                type="number"
                                id="memo-interval"
                                prop:value=min_memo_interval
                                on:input=move |ev| {
                                    let input = event_target::<HtmlInputElement>(&ev);
                                    if let Ok(value) = input.value().parse::<i64>() {
                                        set_min_memo_interval.set(value);
                                    }
                                }
                                min="0"
                                max="86400"
                                prop:disabled=move || is_creating.get()
                            />
                            <small class="form-hint">
                                <i class="fas fa-info-circle"></i>
                                "Minimum time between messages (0-86400 seconds, default: 60)"
                            </small>
                        </div>
                    </div>

                    // Right side: Group Image (Pixel Art) and Burn Amount
                    <div class="form-right">
                        <div class="pixel-art-editor">
                            <div class="pixel-art-header">
                                <label>
                                    <i class="fas fa-image"></i>
                                    "Group Image"
                                </label>
                                <div class="pixel-art-controls">
                                    <select
                                        class="size-selector"
                                        prop:value=move || grid_size.get().to_string()
                                        on:change=move |ev| {
                                            let value = event_target_value(&ev);
                                            if let Ok(size) = value.parse::<usize>() {
                                                set_grid_size.set(size);
                                                set_pixel_art.set(Pixel::new_with_size(size));
                                            }
                                        }
                                        prop:disabled=move || is_creating.get()
                                    >
                                        <option value="16">"16×16 pixels"</option>
                                        <option value="32">"32×32 pixels"</option>
                                    </select>
                                    <button 
                                        type="button"
                                        class="import-btn"
                                        on:click=handle_import
                                        prop:disabled=move || is_creating.get()
                                    >
                                        <i class="fas fa-upload"></i>
                                        "Import Image"
                                    </button>
                                </div>
                            </div>
                            
                            // Pixel Art Canvas
                            {move || {
                                let art_string = pixel_art.get().to_optimal_string();
                                let click_handler = Box::new(move |row, col| {
                                    let mut new_art = pixel_art.get();
                                    new_art.toggle_pixel(row, col);
                                    set_pixel_art.set(new_art);
                                });
                                
                                view! {
                                    <PixelView
                                        art=art_string
                                        size=256
                                        editable=true
                                        show_grid=true
                                        on_click=click_handler
                                    />
                                }
                            }}

                            // Pixel art info
                            <div class="pixel-string-info">
                                <div class="string-display">
                                    <span class="label">
                                        <i class="fas fa-code"></i>
                                        "Encoded String: "
                                    </span>
                                    <span class="value">
                                        {move || {
                                            let art_string = pixel_art.get().to_optimal_string();
                                            if art_string.len() <= 20 {
                                                art_string
                                            } else {
                                                format!("{}...{}", &art_string[..10], &art_string[art_string.len()-10..])
                                            }
                                        }}
                                    </span>
                                    <div class="copy-container">
                                        <button
                                            type="button"
                                            class="copy-button"
                                            on:click=copy_string
                                            title="Copy encoded string to clipboard"
                                        >
                                            <i class="fas fa-copy"></i>
                                        </button>
                                        <div 
                                            class="copy-tooltip"
                                            class:show=move || show_copied.get()
                                        >
                                            "Copied!"
                                        </div>
                                    </div>
                                </div>
                                <div class="string-length">
                                    <span class="label">
                                        <i class="fas fa-ruler"></i>
                                        "Length: "
                                    </span>
                                    <span class="value">
                                        {move || format!("{} bytes", pixel_art.get().to_optimal_string().len())}
                                    </span>
                                </div>
                            </div>
                        </div>
                        
                        // Burn Amount - moved to right side
                        <div class="form-group" style="margin-top: 20px;">
                            <label for="burn-amount">
                                <i class="fas fa-fire"></i>
                                "Burn Amount (MEMO tokens)"
                            </label>
                            <input
                                type="number"
                                id="burn-amount"
                                prop:value=burn_amount
                                on:input=move |ev| {
                                    let input = event_target::<HtmlInputElement>(&ev);
                                    if let Ok(value) = input.value().parse::<u64>() {
                                        set_burn_amount.set(value);
                                    }
                                }
                                min="42069"
                                prop:disabled=move || is_creating.get()
                            />
                            <small class="form-hint">
                                <i class="fas fa-wallet"></i>
                                {move || {
                                    let balance = session.with(|s| s.get_token_balance());
                                    let is_sufficient = balance >= 42069.0;
                                    view! {
                                        "Minimum: 42,069 MEMO tokens (Available: "
                                        <span class={if is_sufficient { "balance-sufficient" } else { "balance-insufficient" }}>
                                            {format!("{:.2} MEMO", balance)}
                                        </span>
                                        ")"
                                    }
                                }}
                            </small>
                        </div>
                    </div>
                </div>

                // Memo size indicator
                <div class="memo-size-indicator">
                    <div class="size-info">
                        <span class="size-label">
                            <i class="fas fa-database"></i>
                            "Memo Size: "
                        </span>
                        {move || {
                            let (size, is_valid, status) = calculate_memo_size();
                            view! {
                                <span class="size-value" class:valid=is_valid class:invalid=move || !is_valid>
                                    {format!("{} bytes", size)}
                                </span>
                                <span class="size-range">" (Required: 69-800 bytes)"</span>
                                <span class="size-status" class:valid=is_valid class:invalid=move || !is_valid>
                                    {status}
                                </span>
                            }
                        }}
                    </div>
                    <div class="size-progress">
                        {move || {
                            let (size, is_valid, _) = calculate_memo_size();
                            let percentage = ((size as f64 / 800.0) * 100.0).min(100.0);
                            
                            view! {
                                <div class="progress-bar">
                                    <div class="progress-track">
                                        <div 
                                            class="progress-fill"
                                            class:valid=is_valid
                                            class:invalid=move || !is_valid
                                            style:width=move || format!("{}%", percentage)
                                        ></div>
                                        <div class="progress-markers">
                                            <div class="marker min-marker" style="left: 8.625%"></div>
                                            <div class="marker max-marker" style="left: 100%"></div>
                                        </div>
                                    </div>
                                </div>
                            }
                        }}
                    </div>
                </div>

                // Error message
                {move || {
                    let message = error_message.get();
                    if !message.is_empty() {
                        view! {
                            <div class="error-message" 
                                class:success=message.contains("✅")
                                class:error=message.contains("❌")
                            >
                                {message}
                            </div>
                        }
                    } else {
                        view! { <div></div> }
                    }
                }}

                // Creating status
                {move || {
                    let status = creating_status.get();
                    if !status.is_empty() {
                        view! {
                            <div class="creating-progress">
                                <i class="fas fa-spinner fa-spin"></i>
                                <span>{status}</span>
                            </div>
                        }
                    } else {
                        view! { <div></div> }
                    }
                }}

                // Submit button
                <div class="button-group">
                    <button
                        type="submit"
                        class="create-group-btn"
                        prop:disabled=move || {
                            is_creating.get() ||
                            group_name.get().trim().is_empty() ||
                            group_name.get().len() > 64 ||
                            group_description.get().len() > 128 ||
                            parse_tags().len() > 4 ||
                            min_memo_interval.get() < 0 ||
                            min_memo_interval.get() > 86400 ||
                            burn_amount.get() < 42069 ||
                            session.with(|s| s.get_token_balance()) < burn_amount.get() as f64
                        }
                    >
                        <i class="fas fa-rocket"></i>
                        {move || {
                            if is_creating.get() {
                                "Creating Group...".to_string()
                            } else {
                                format!("Create Group (Burn {} MEMO)", burn_amount.get())
                            }
                        }}
                    </button>
                </div>
            </form>
        </div>
    }
} 