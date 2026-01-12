use leptos::*;
use crate::core::session::Session;
use crate::core::rpc_forum::PostReply;
use crate::core::rpc_base::RpcConnection;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlInputElement, FileReader, ProgressEvent};
use js_sys::Uint8Array;
use std::rc::Rc;
use crate::pages::pixel_view::{PixelView, LazyPixelView};
use crate::core::pixel::Pixel;

/// Post row data for table display
#[derive(Clone, Debug, PartialEq)]
struct PostRow {
    post_id: u64,
    title: String,
    content: String,
    image: String,
    creator: String,
    burned_amount: u64,
    last_reply_time: i64,
    reply_count: u64,
    created_at: i64,
}

/// Page view state
#[derive(Clone, Debug, PartialEq)]
enum PageView {
    PostList,
    PostDetails(PostRow),
}

/// Forum page component - displays forum posts
#[component]
pub fn ForumPage(
    session: RwSignal<Session>,
) -> impl IntoView {
    let (posts, set_posts) = create_signal::<Vec<PostRow>>(vec![]);
    let (loading, set_loading) = create_signal(true);
    let (error_message, set_error_message) = create_signal::<Option<String>>(None);
    
    // Page navigation state
    let (current_view, set_current_view) = create_signal(PageView::PostList);
    
    // Create Post Dialog states
    let (show_create_dialog, set_show_create_dialog) = create_signal(false);
    
    // Countdown state
    let (countdown_seconds, set_countdown_seconds) = create_signal::<Option<i32>>(None);

    // Function to load/refresh posts data  
    let load_posts_data = create_action(move |_: &()| {
        let session_clone = session;
        async move {
            set_loading.set(true);
            set_error_message.set(None);
            
            let rpc = RpcConnection::new();
            
            match rpc.get_all_forum_posts().await {
                Ok(stats) => {
                    log::info!("Fetched {} forum posts", stats.posts.len());
                    
                    let mut post_rows: Vec<PostRow> = stats.posts.into_iter().map(|post| {
                        PostRow {
                            post_id: post.post_id,
                            title: post.title,
                            content: post.content,
                            image: post.image,
                            creator: post.creator,
                            burned_amount: post.burned_amount,
                            last_reply_time: post.last_reply_time,
                            reply_count: post.reply_count,
                            created_at: post.created_at,
                        }
                    }).collect();
                    
                    // Sort posts:
                    // 1. By burned_amount descending (more burns first)
                    // 2. If equal, by last_reply_time descending (newer first)
                    post_rows.sort_by(|a, b| {
                        match b.burned_amount.cmp(&a.burned_amount) {
                            std::cmp::Ordering::Equal => b.last_reply_time.cmp(&a.last_reply_time),
                            other => other,
                        }
                    });
                    
                    set_posts.set(post_rows);
                },
                Err(e) => {
                    log::error!("Failed to fetch forum posts: {}", e);
                    set_error_message.set(Some(format!("Failed to load posts: {}", e)));
                }
            }
            
            set_loading.set(false);
        }
    });

    // Load posts on component mount
    create_effect(move |_| {
        load_posts_data.dispatch(());
    });

    // Function to open create post dialog
    let open_create_dialog = move |_| {
        set_show_create_dialog.set(true);
    };

    // Function to close create post dialog
    let close_create_dialog = move || {
        set_show_create_dialog.set(false);
    };

    // Function to view post details
    let view_post_details = move |post: PostRow| {
        set_current_view.set(PageView::PostDetails(post));
    };

    // Function to go back to post list
    let back_to_post_list = move || {
        set_current_view.set(PageView::PostList);
    };

    // Function to handle successful post creation
    let on_post_created = move |signature: String, post_id: u64| {
        log::info!("Post created successfully! ID: {}, Signature: {}", post_id, signature);
        set_show_create_dialog.set(false);
        
        // start 20 seconds countdown
        set_countdown_seconds.set(Some(20));
        spawn_local(async move {
            for i in (1..=20).rev() {
                TimeoutFuture::new(1000).await;
                set_countdown_seconds.set(Some(i - 1));
            }
            set_countdown_seconds.set(None);
        });
        
        // Wait 20 seconds before refreshing to allow blockchain to update
        spawn_local(async move {
            log::info!("Waiting 20 seconds for blockchain to update...");
            TimeoutFuture::new(20_000).await;
            
            log::info!("Refreshing post list after post creation...");
            load_posts_data.dispatch(());
        });
    };

    // Function to handle post creation error
    let on_post_creation_error = move |error: String| {
        log::error!("Post creation failed: {}", error);
    };

    view! {
        <div class="forum-page">
            {move || {
                match current_view.get() {
                    PageView::PostList => {
                        view! {
                            <div class="post-list-view">
                                <div class="forum-header">
                                    <div class="header-content">
                                        <div class="header-text">
                                            <h1>
                                                <i class="fas fa-users"></i>
                                                "Forum"
                                            </h1>
                                            <p class="forum-subtitle">"Community discussions powered by MEMO tokens"</p>
                                        </div>
                                        <div class="header-actions">
                                            <button 
                                                class="new-post-button"
                                                on:click=open_create_dialog
                                                disabled=move || loading.get()
                                                title="Create new post"
                                            >
                                                <i class="fas fa-plus"></i>
                                                "New Post"
                                            </button>
                                        </div>
                                    </div>
                                </div>
                                
                                // countdown banner display
                                <Show when=move || countdown_seconds.get().is_some()>
                                    <div class="countdown-banner">
                                        <div class="countdown-content">
                                            <i class="fas fa-clock"></i>
                                            <span>
                                                "Post created successfully! List will refresh in "
                                                <strong>{move || countdown_seconds.get().unwrap_or(0).to_string()}</strong>
                                                " seconds..."
                                            </span>
                                        </div>
                                    </div>
                                </Show>
                                
                                <div class="forum-content">
                                    {move || {
                                        if loading.get() {
                                            view! {
                                                <div class="loading-state">
                                                    <p>"Loading posts..."</p>
                                                </div>
                                            }.into_view()
                                        } else if let Some(error) = error_message.get() {
                                            view! {
                                                <div class="error-state">
                                                    <p>"Error: "{error}</p>
                                                </div>
                                            }.into_view()
                                        } else {
                                            let post_list = posts.get();
                                            if post_list.is_empty() {
                                                view! {
                                                    <div class="empty-state">
                                                        <i class="fas fa-comments"></i>
                                                        <p>"No posts yet. Be the first to create a post!"</p>
                                                    </div>
                                                }.into_view()
                                            } else {
                                                view! {
                                                    <div class="post-table-container">
                                                        <table class="post-table">
                                                            <thead>
                                                                <tr>
                                                                    <th class="rank-col">"#"</th>
                                                                    <th class="title-col">"Title"</th>
                                                                    <th class="burned-col">"Burned (MEMO)"</th>
                                                                    <th class="replies-col">"Replies"</th>
                                                                    <th class="time-col">"Last Activity"</th>
                                                                    <th class="action-col">"Action"</th>
                                                                </tr>
                                                            </thead>
                                                            <tbody>
                                                                {post_list.into_iter().enumerate().map(|(index, post)| {
                                                                    let burned_tokens = post.burned_amount / 1_000_000;
                                                                    let post_clone = post.clone();
                                                                    let rank = index + 1;
                                                                    
                                                                    let last_activity = if post.last_reply_time > 0 {
                                                                        format_timestamp(post.last_reply_time)
                                                                    } else {
                                                                        format_timestamp(post.created_at)
                                                                    };
                                                                    
                                                                    view! {
                                                                        <tr class="post-row">
                                                                            <td class="rank-cell">
                                                                                {
                                                                                    if rank == 1 {
                                                                                        view! {
                                                                                            <span class="rank-icon rank-1st">
                                                                                                <i class="fas fa-trophy"></i>
                                                                                            </span>
                                                                                        }.into_view()
                                                                                    } else if rank == 2 {
                                                                                        view! {
                                                                                            <span class="rank-icon rank-2nd">
                                                                                                <i class="fas fa-medal"></i>
                                                                                            </span>
                                                                                        }.into_view()
                                                                                    } else if rank == 3 {
                                                                                        view! {
                                                                                            <span class="rank-icon rank-3rd">
                                                                                                <i class="fas fa-medal"></i>
                                                                                            </span>
                                                                                        }.into_view()
                                                                                    } else {
                                                                                        view! {
                                                                                            <span class="rank-number">{rank.to_string()}</span>
                                                                                        }.into_view()
                                                                                    }
                                                                                }
                                                                            </td>
                                                                            <td class="title-cell">
                                                                                <div class="post-title-content">
                                                                                    {if !post.image.is_empty() {
                                                                                        if post.image.starts_with("c:") || post.image.starts_with("n:") {
                                                                                            view! {
                                                                                                <div class="post-avatar-small">
                                                                                                    <LazyPixelView
                                                                                                        art={post.image.clone()}
                                                                                                        size=32
                                                                                                    />
                                                                                                </div>
                                                                                            }.into_view()
                                                                                        } else {
                                                                                            view! {
                                                                                                <div class="post-avatar-small">
                                                                                                    <img src={post.image.clone()} alt="Post" />
                                                                                                </div>
                                                                                            }.into_view()
                                                                                        }
                                                                                    } else {
                                                                                        view! {
                                                                                            <div class="post-avatar-small placeholder">
                                                                                                <i class="fas fa-file-alt"></i>
                                                                                            </div>
                                                                                        }.into_view()
                                                                                    }}
                                                                                    <span class="post-title">{post.title}</span>
                                                                                </div>
                                                                            </td>
                                                                            <td class="burned-cell">
                                                                                <span class="burned-amount">
                                                                                    <i class="fas fa-fire"></i>
                                                                                    {format_number_with_commas(burned_tokens)}
                                                                                </span>
                                                                            </td>
                                                                            <td class="replies-cell">
                                                                                <span class="reply-count">
                                                                                    <i class="fas fa-comment"></i>
                                                                                    {post.reply_count.to_string()}
                                                                                </span>
                                                                            </td>
                                                                            <td class="time-cell">
                                                                                <span class="last-activity">{last_activity}</span>
                                                                            </td>
                                                                            <td class="action-cell">
                                                                                <button 
                                                                                    class="view-button"
                                                                                    on:click=move |_| view_post_details(post_clone.clone())
                                                                                    title="View post"
                                                                                >
                                                                                    <i class="fas fa-arrow-right"></i>
                                                                                    "View"
                                                                                </button>
                                                                            </td>
                                                                        </tr>
                                                                    }
                                                                }).collect::<Vec<_>>()}
                                                            </tbody>
                                                        </table>
                                                    </div>
                                                }.into_view()
                                            }
                                        }
                                    }}
                                </div>
                            </div>
                        }.into_view()
                    },
                    PageView::PostDetails(post) => {
                        view! {
                            <PostDetailsView
                                post=post
                                on_back=Rc::new(back_to_post_list)
                                session=session
                            />
                        }.into_view()
                    }
                }
            }}

            // Create Post Dialog
            <Show when=move || show_create_dialog.get()>
                <div class="modal-overlay">
                    <CreatePostForm
                        session=session
                        on_close=Rc::new(close_create_dialog)
                        on_success=Rc::new(on_post_created)
                        on_error=Rc::new(on_post_creation_error)
                    />
                </div>
            </Show>
        </div>
    }
}

/// Format timestamp to human-readable format
fn format_timestamp(timestamp: i64) -> String {
    if timestamp <= 0 {
        return "Never".to_string();
    }
    
    let date = web_sys::js_sys::Date::new(&JsValue::from_f64(timestamp as f64 * 1000.0));
    format!(
        "{}-{:02}-{:02} {:02}:{:02}",
        date.get_full_year(),
        date.get_month() + 1,
        date.get_date(),
        date.get_hours(),
        date.get_minutes()
    )
}

/// Format number with commas for readability
fn format_number_with_commas(num: u64) -> String {
    let s = num.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }
    result
}

/// Shorten address for display
fn shorten_address(addr: &str) -> String {
    if addr.len() > 12 {
        format!("{}...{}", &addr[..6], &addr[addr.len()-4..])
    } else {
        addr.to_string()
    }
}

/// Parse message content - handles both JSON format and plain text
/// Uses custom JSON parsing to preserve newlines and handle control characters
/// Returns (title, content, image)
fn parse_message_content(message: &str) -> (String, String, String) {
    // Clean NULL bytes but preserve newlines and other whitespace
    let cleaned: String = message
        .chars()
        .filter(|c| *c != '\0')  // Only remove NULL bytes
        .collect();
    let cleaned = cleaned.trim();
    
    // Check if it looks like JSON
    if cleaned.starts_with('{') {
        // Use custom JSON field extraction to handle unescaped newlines
        let title = extract_json_field(cleaned, "title").unwrap_or_default();
        let content = extract_json_field(cleaned, "content")
            .or_else(|| extract_json_field(cleaned, "message"))
            .unwrap_or_default();
        let image = extract_json_field(cleaned, "image").unwrap_or_default();
        
        return (title, content, image);
    }
    
    // Check for [title] content [img:xxx] format (from reply form)
    if cleaned.starts_with('[') {
        if let Some(title_end) = cleaned.find(']') {
            let title = cleaned[1..title_end].to_string();
            let rest = cleaned[title_end + 1..].trim();
            
            // Check for [img:xxx] suffix
            if let Some(img_start) = rest.rfind("[img:") {
                let content = rest[..img_start].trim().to_string();
                let img_part = &rest[img_start + 5..];
                let image = if let Some(img_end) = img_part.find(']') {
                    img_part[..img_end].to_string()
                } else {
                    String::new()
                };
                return (title, content, image);
            } else {
                return (title, rest.to_string(), String::new());
            }
        }
    }
    
    // Plain text message - no title, content is the message, no image
    (String::new(), cleaned.to_string(), String::new())
}

/// Extract a field value from JSON string (handles unescaped newlines)
fn extract_json_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", field);
    let start = json.find(&pattern)? + pattern.len();
    let remaining = &json[start..];
    
    // Find the closing quote, handling escaped quotes
    let mut end = 0;
    let mut escaped = false;
    for (i, c) in remaining.chars().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' {
            end = i;
            break;
        }
    }
    
    let value = &remaining[..end];
    // Unescape the string - handle escaped quotes and backslashes, and \n sequences
    Some(value
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\"))
}


/// Post Details View component
#[component]
fn PostDetailsView(
    post: PostRow,
    on_back: Rc<dyn Fn()>,
    session: RwSignal<Session>,
) -> impl IntoView {
    let on_back_signal = create_rw_signal(Some(on_back));
    
    // Store post data as reactive signal for updates
    let post_data = create_rw_signal(post.clone());
    let current_post = move || post_data.get();
    
    let handle_back = move |_| {
        on_back_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };
    
    // Replies state
    let (replies, set_replies) = create_signal::<Vec<PostReply>>(vec![]);
    let (replies_loading, set_replies_loading) = create_signal(true);
    let (replies_error, set_replies_error) = create_signal::<Option<String>>(None);
    
    // Reply dialog state
    let (show_reply_dialog, set_show_reply_dialog) = create_signal(false);
    let (initial_reply_type, set_initial_reply_type) = create_signal("burn".to_string()); // "burn" or "mint"
    
    // Refresh trigger
    let (refresh_trigger, set_refresh_trigger) = create_signal(0u32);
    
    // Load replies
    {
        let post_id = post.post_id;
        create_effect(move |_| {
            let _ = refresh_trigger.get();
            
            spawn_local(async move {
                set_replies_loading.set(true);
                set_replies_error.set(None);
                
                let rpc = RpcConnection::new();
                match rpc.get_post_replies(post_id, 100, None).await {
                    Ok(response) => {
                        log::info!("Loaded {} replies for post {}", response.replies.len(), post_id);
                        set_replies.set(response.replies);
                    },
                    Err(e) => {
                        log::error!("Failed to load replies: {}", e);
                        set_replies_error.set(Some(format!("Failed to load replies: {}", e)));
                    }
                }
                set_replies_loading.set(false);
            });
        });
    }
    
    // Computed values
    let burned_display = move || {
        let post = current_post();
        let burned_tokens = post.burned_amount / 1_000_000;
        format_number_with_commas(burned_tokens)
    };
    
    let created_at_display = move || {
        let post = current_post();
        format_timestamp(post.created_at)
    };
    
    let last_activity_display = move || {
        let post = current_post();
        if post.last_reply_time > 0 {
            format_timestamp(post.last_reply_time)
        } else {
            format_timestamp(post.created_at)
        }
    };
    
    // Open reply dialog
    let open_reply_dialog = move |_| {
        set_initial_reply_type.set("mint".to_string());
        set_show_reply_dialog.set(true);
    };
    
    // Close reply dialog
    let close_reply_dialog = move || {
        set_show_reply_dialog.set(false);
    };
    
    // Handle reply success
    let on_reply_success = {
        let post_id = post.post_id;
        move |_signature: String| {
            log::info!("Reply posted successfully!");
            set_show_reply_dialog.set(false);
            
            // Refresh immediately
            set_refresh_trigger.update(|n| *n += 1);
            
            // Also refresh post data immediately
            {
                let rpc = RpcConnection::new();
                spawn_local(async move {
                    if let Ok(post_info) = rpc.get_post_info(post_id).await {
                        post_data.update(|p| {
                            p.burned_amount = post_info.burned_amount;
                            p.reply_count = post_info.reply_count;
                            p.last_reply_time = post_info.last_reply_time;
                        });
                    }
                });
            }
            
            // Refresh again after 3 seconds to catch confirmed transaction
            spawn_local(async move {
                TimeoutFuture::new(3_000).await;
                set_refresh_trigger.update(|n| *n += 1);
                
                // Refresh post data again
                let rpc = RpcConnection::new();
                if let Ok(post_info) = rpc.get_post_info(post_id).await {
                    post_data.update(|p| {
                        p.burned_amount = post_info.burned_amount;
                        p.reply_count = post_info.reply_count;
                        p.last_reply_time = post_info.last_reply_time;
                    });
                }
            });
        }
    };
    
    // Creator display
    let (creator_display, set_creator_display) = create_signal(shorten_address(&post.creator));
    
    // Fetch creator profile
    {
        let creator_addr = post.creator.clone();
        create_effect(move |_| {
            let addr = creator_addr.clone();
            spawn_local(async move {
                let rpc = RpcConnection::new();
                if let Ok(Some(profile)) = rpc.get_profile(&addr).await {
                    set_creator_display.set(profile.username);
                }
            });
        });
    }

    view! {
        <div class="post-details-page">
            <div class="post-details-container">
                // Back button
                <button 
                    class="back-btn"
                    on:click=handle_back
                    title="Back to forum"
                >
                    <i class="fas fa-arrow-left"></i>
                    "Back to Forum"
                </button>
                
                // Post Card
                <div class="post-detail-card">
                    // Card Header
                    <div class="post-card-header">
                        <div class="post-header-content">
                            // Post Image
                            {move || {
                                let post = current_post();
                                if !post.image.is_empty() {
                                    if post.image.starts_with("c:") || post.image.starts_with("n:") {
                                        view! {
                                            <div class="post-avatar-large">
                                                <LazyPixelView
                                                    art={post.image.clone()}
                                                    size=80
                                                />
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <div class="post-avatar-large">
                                                <img src={post.image.clone()} alt="Post" />
                                            </div>
                                        }.into_view()
                                    }
                                } else {
                                    view! {
                                        <div class="post-avatar-large placeholder">
                                            <i class="fas fa-file-alt"></i>
                                        </div>
                                    }.into_view()
                                }
                            }}
                            
                            // Title and Creator
                            <div class="post-title-section">
                                <h1 class="post-detail-title">{move || current_post().title}</h1>
                                <div class="post-creator">
                                    <i class="fas fa-user"></i>
                                    <span>{creator_display}</span>
                                </div>
                            </div>
                            
                            // Reply button
                            <div class="post-action-buttons">
                                <button 
                                    class="reply-btn"
                                    on:click=open_reply_dialog
                                    title="Reply to post"
                                >
                                    <i class="fas fa-reply"></i>
                                    "Reply"
                                </button>
                            </div>
                        </div>
                    </div>
                    
                    // Card Body - Post Content
                    <div class="post-card-body">
                        <div class="post-content-section">
                            <p class="post-content" inner_html={move || {
                                let content = current_post().content;
                                let (_, parsed_content, _) = parse_message_content(&content);
                                // Convert newlines to <br> for HTML display
                                if parsed_content.is_empty() {
                                    content.replace('\n', "<br>")
                                } else {
                                    parsed_content.replace('\n', "<br>")
                                }
                            }}></p>
                        </div>
                        
                        // Stats
                        <div class="post-stats">
                            <div class="stat-item">
                                <i class="fas fa-fire"></i>
                                <span class="stat-value">{burned_display}</span>
                                <span class="stat-label">"MEMO Burned"</span>
                            </div>
                            <div class="stat-item">
                                <i class="fas fa-comment"></i>
                                <span class="stat-value">{move || current_post().reply_count.to_string()}</span>
                                <span class="stat-label">"Replies"</span>
                            </div>
                            <div class="stat-item">
                                <i class="fas fa-calendar"></i>
                                <span class="stat-value">{created_at_display}</span>
                                <span class="stat-label">"Created"</span>
                            </div>
                            <div class="stat-item">
                                <i class="fas fa-clock"></i>
                                <span class="stat-value">{last_activity_display}</span>
                                <span class="stat-label">"Last Activity"</span>
                            </div>
                        </div>
                    </div>
                </div>
                
                // Replies Section
                <div class="replies-section">
                    <div class="replies-header">
                        <h2>
                            <i class="fas fa-comments"></i>
                            "Replies"
                        </h2>
                    </div>
                    
                    <div class="replies-list">
                        {move || {
                            if replies_loading.get() {
                                view! {
                                    <div class="loading-state">
                                        <p>"Loading replies..."</p>
                                    </div>
                                }.into_view()
                            } else if let Some(error) = replies_error.get() {
                                view! {
                                    <div class="error-state">
                                        <p>{error}</p>
                                    </div>
                                }.into_view()
                            } else {
                                let reply_list = replies.get();
                                if reply_list.is_empty() {
                                    view! {
                                        <div class="empty-state">
                                            <i class="fas fa-comment-slash"></i>
                                            <p>"No replies yet. Be the first to reply!"</p>
                                        </div>
                                    }.into_view()
                                } else {
                                    view! {
                                        <div class="reply-cards">
                                            {reply_list.into_iter().map(|reply| {
                                                let reply_type_class = if reply.is_mint { "mint-reply" } else { "burn-reply" };
                                                let reply_type_icon = if reply.is_mint { "fa-coins" } else { "fa-fire" };
                                                let reply_type_text = if reply.is_mint { "Minted" } else { "Burned" };
                                                let burn_display = if reply.is_mint {
                                                    "1 MEMO (mint)".to_string()
                                                } else {
                                                    format!("{} MEMO", reply.burn_amount / 1_000_000)
                                                };
                                                
                                                view! {
                                                    <div class={format!("reply-card {}", reply_type_class)}>
                                                        <div class="reply-header">
                                                            <div class="reply-user">
                                                                <i class="fas fa-user"></i>
                                                                <span>{shorten_address(&reply.user)}</span>
                                                            </div>
                                                            <div class="reply-meta">
                                                                <span class="reply-type">
                                                                    <i class={format!("fas {}", reply_type_icon)}></i>
                                                                    {reply_type_text}
                                                                </span>
                                                                <span class="reply-amount">{burn_display}</span>
                                                                <span class="reply-time">{format_timestamp(reply.timestamp)}</span>
                                                            </div>
                                                        </div>
                                                        <div class="reply-content">
                                                            {
                                                                let msg = reply.message.clone();
                                                                if msg.is_empty() {
                                                                    view! { <p class="no-message">"(No message)"</p> }.into_view()
                                                                } else {
                                                                    let (title, content, image) = parse_message_content(&msg);
                                                                    // Don't fallback to raw message - if content is empty, show empty
                                                                    // The title and image are displayed separately
                                                                    let has_title = !title.is_empty();
                                                                    let has_content = !content.is_empty();
                                                                    let has_image = !image.is_empty() && image != "n:" && image != "n:16";
                                                                    
                                                                    view! {
                                                                        <div class="reply-body">
                                                                            // Image on left side
                                                                            {if has_image {
                                                                                if image.starts_with("c:") || image.starts_with("n:") {
                                                                                    view! {
                                                                                        <div class="reply-image">
                                                                                            <LazyPixelView
                                                                                                art={image}
                                                                                                size=64
                                                                                            />
                                                                                        </div>
                                                                                    }.into_view()
                                                                                } else {
                                                                                    view! {
                                                                                        <div class="reply-image">
                                                                                            <img src={image} alt="Reply image" />
                                                                                        </div>
                                                                                    }.into_view()
                                                                                }
                                                                            } else {
                                                                                view! { <span></span> }.into_view()
                                                                            }}
                                                                            // Title and content on right side
                                                                            <div class="reply-text">
                                                                                {if has_title {
                                                                                    view! {
                                                                                        <h4 class="reply-title">{title}</h4>
                                                                                    }.into_view()
                                                                                } else {
                                                                                    view! { <span></span> }.into_view()
                                                                                }}
                                                                                {if has_content {
                                                                                    view! {
                                                                                        <p inner_html={content.replace('\n', "<br>")}></p>
                                                                                    }.into_view()
                                                                                } else {
                                                                                    view! { <span></span> }.into_view()
                                                                                }}
                                                                            </div>
                                                                        </div>
                                                                    }.into_view()
                                                                }
                                                            }
                                                        </div>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_view()
                                }
                            }
                        }}
                    </div>
                </div>
            </div>
            
            // Reply Dialog
            <Show when=move || show_reply_dialog.get()>
                <div class="modal-overlay">
                    <ReplyPostForm
                        session=session
                        post_id=post.post_id
                        initial_type=initial_reply_type.get()
                        on_close=Rc::new(close_reply_dialog)
                        on_success=Rc::new(on_reply_success.clone())
                    />
                </div>
            </Show>
        </div>
    }
}

/// Create Post Form component - strictly reference project page design
#[component]
fn CreatePostForm(
    session: RwSignal<Session>,
    on_close: Rc<dyn Fn()>,
    on_success: Rc<dyn Fn(String, u64)>,
    on_error: Rc<dyn Fn(String)>,
) -> impl IntoView {
    let on_close_signal = create_rw_signal(Some(on_close));
    let on_success_signal = create_rw_signal(Some(on_success));
    let on_error_signal = create_rw_signal(Some(on_error));
    
    // Form state
    let (title, set_title) = create_signal(String::new());
    let (content, set_content) = create_signal(String::new());
    let (burn_amount, set_burn_amount) = create_signal(1u64);
    let (is_submitting, set_is_submitting) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    let (show_copied, set_show_copied) = create_signal(false);
    
    // Pixel art state - use Pixel struct like project_page does
    let (pixel_art, set_pixel_art) = create_signal(Pixel::new_with_size(16));
    let (grid_size, set_grid_size) = create_signal(16usize);
    
    // Get token balance
    let token_balance = move || session.with(|s| s.get_token_balance());
    
    // Get image data
    let get_image_data = move || -> String {
        pixel_art.get().to_optimal_string()
    };
    
    // Get creator pubkey
    let get_creator_pubkey = move || -> String {
        session.with(|s| s.get_public_key().unwrap_or_default())
    };
    
    // Calculate memo size in real time (69-800 bytes)
    let calculate_memo_size = move || -> (usize, bool, String) {
        use crate::core::rpc_forum::ForumConfig;
        
        let title_val = title.get();
        let content_val = content.get();
        let image_val = get_image_data();
        let creator_val = get_creator_pubkey();
        let burn_val = burn_amount.get() * 1_000_000;
        
        // Estimate memo size using ForumConfig
        let estimated_size = ForumConfig::estimate_create_post_memo_size(
            &creator_val,
            0, // post_id placeholder
            &title_val,
            &content_val,
            &image_val,
            burn_val,
        );
        
        let is_valid = estimated_size >= 69 && estimated_size <= 800;
        let status = if estimated_size < 69 {
            "Too short".to_string()
        } else if estimated_size > 800 {
            "Too long".to_string()
        } else {
            "Valid".to_string()
        };
        
        (estimated_size, is_valid, status)
    };
    
    let handle_close = move |_: web_sys::MouseEvent| {
        on_close_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };
    
    let handle_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        
        let title_val = title.get();
        let content_val = content.get();
        let burn_val = burn_amount.get();
        let image_val = pixel_art.get().to_optimal_string();
        
        // Validation
        if title_val.trim().is_empty() {
            set_error_message.set("Title is required".to_string());
            return;
        }
        if title_val.len() > 128 {
            set_error_message.set("Title must be 128 characters or less".to_string());
            return;
        }
        if content_val.trim().is_empty() {
            set_error_message.set("Content is required".to_string());
            return;
        }
        if content_val.len() > 512 {
            set_error_message.set("Content must be 512 characters or less".to_string());
            return;
        }
        if burn_val < 1 {
            set_error_message.set("Burn amount must be at least 1 MEMO".to_string());
            return;
        }
        
        let burn_lamports = burn_val * 1_000_000;
        
        // Check balance
        if token_balance() < burn_val as f64 {
            set_error_message.set("Insufficient MEMO balance".to_string());
            return;
        }
        
        // Check memo size
        let (_, is_valid, _) = calculate_memo_size();
        if !is_valid {
            set_error_message.set("Memo size is invalid (must be 69-800 bytes)".to_string());
            return;
        }
        
        set_is_submitting.set(true);
        set_error_message.set(String::new());
        
        let session_clone = session;
        let on_success_signal = on_success_signal.clone();
        let on_error_signal = on_error_signal.clone();
        
        spawn_local(async move {
            let mut session_update = session_clone.get_untracked();
            
            match session_update.create_forum_post(&title_val, &content_val, &image_val, burn_lamports).await {
                Ok((signature, post_id)) => {
                    log::info!("Post created: {}, ID: {}", signature, post_id);
                    
                    // Update session to trigger balance refresh
                    session_clone.update(|s| {
                        s.mark_balance_update_needed();
                    });
                    
                    on_success_signal.with_untracked(|cb_opt| {
                        if let Some(callback) = cb_opt.as_ref() {
                            callback(signature, post_id);
                        }
                    });
                },
                Err(e) => {
                    log::error!("Failed to create post: {}", e);
                    set_error_message.set(format!("Failed to create post: {}", e));
                    set_is_submitting.set(false);
                    
                    on_error_signal.with_untracked(|cb_opt| {
                        if let Some(callback) = cb_opt.as_ref() {
                            callback(e.to_string());
                        }
                    });
                }
            }
        });
    };
    
    // Copy pixel art string
    let copy_string = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        let art_string = pixel_art.get().to_optimal_string();
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(&art_string);
            set_show_copied.set(true);
            spawn_local(async move {
                TimeoutFuture::new(2000).await;
                set_show_copied.set(false);
            });
        }
    };
    
    // Handle image import
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
                let current_grid_size = grid_size_signal.get();
                
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

    view! {
        <div class="create-post-form">
            // Header with title and close button
            <div class="form-header">
                <h3 class="form-title">
                    <i class="fas fa-edit"></i>
                    "Create New Post"
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
            
            <form class="project-form" on:submit=handle_submit>
                <div class="form-layout">
                    // Left side: Basic Information
                    <div class="form-left">
                        // Post Title
                        <div class="form-group">
                            <label for="post-title">
                                <i class="fas fa-heading"></i>
                                "Post Title"
                                <span class="required">"*"</span>
                            </label>
                            <input
                                type="text"
                                id="post-title"
                                prop:value=title
                                on:input=move |ev| set_title.set(event_target_value(&ev))
                                placeholder="Enter post title (1-128 characters)..."
                                maxlength="128"
                                prop:disabled=move || is_submitting.get()
                            />
                            <small class="char-count">
                                {move || format!("{}/128 characters", title.get().len())}
                            </small>
                        </div>

                        // Post Content
                        <div class="form-group">
                            <label for="post-content">
                                <i class="fas fa-align-left"></i>
                                "Content"
                                <span class="required">"*"</span>
                            </label>
                            <textarea
                                id="post-content"
                                prop:value=content
                                on:input=move |ev| set_content.set(event_target_value(&ev))
                                placeholder="Write your post content here (max 512 characters)..."
                                maxlength="512"
                                rows="6"
                                prop:disabled=move || is_submitting.get()
                            ></textarea>
                            <small class="char-count">
                                {move || format!("{}/512 characters", content.get().len())}
                            </small>
                        </div>
                    </div>

                    // Right side: Image and Burn Amount
                    <div class="form-right">
                        <div class="pixel-art-editor">
                            <div class="pixel-art-header">
                                <label>
                                    <i class="fas fa-image"></i>
                                    "Post Image (Optional)"
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
                                        prop:disabled=move || is_submitting.get()
                                    >
                                        <option value="16">"16×16 pixels"</option>
                                        <option value="32">"32×32 pixels"</option>
                                    </select>
                                    <button 
                                        type="button"
                                        class="import-btn"
                                        on:click=handle_import
                                        prop:disabled=move || is_submitting.get()
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
                                        size=180
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
                                        "Encoded: "
                                    </span>
                                    <span class="value">
                                        {move || {
                                            let art_string = pixel_art.get().to_optimal_string();
                                            if art_string.len() <= 16 {
                                                art_string
                                            } else {
                                                format!("{}...{}", &art_string[..8], &art_string[art_string.len()-6..])
                                            }
                                        }}
                                    </span>
                                    <div class="copy-container">
                                        <button
                                            type="button"
                                            class="copy-button"
                                            on:click=copy_string
                                            title="Copy"
                                        >
                                            <i class="fas fa-copy"></i>
                                        </button>
                                        <div class="copy-tooltip" class:show=move || show_copied.get()>
                                            "Copied!"
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                        
                        // Burn Amount
                        <div class="form-group" style="margin-top: 16px;">
                            <label for="post-burn-amount">
                                <i class="fas fa-fire"></i>
                                "Burn Amount (MEMO)"
                            </label>
                            <input
                                type="number"
                                id="post-burn-amount"
                                prop:value=burn_amount
                                on:input=move |ev| {
                                    if let Ok(value) = event_target_value(&ev).parse::<u64>() {
                                        set_burn_amount.set(value.max(1));
                                    }
                                }
                                min="1"
                                prop:disabled=move || is_submitting.get()
                            />
                            <small class="form-hint">
                                <i class="fas fa-wallet"></i>
                                {move || {
                                    let balance = session.with(|s| s.get_token_balance());
                                    view! {
                                        "Minimum: 1 MEMO (Available: "
                                        <span class={if balance >= 1.0 { "balance-sufficient" } else { "balance-insufficient" }}>
                                            {format!("{:.2} MEMO", balance)}
                                        </span>
                                        ")"
                                    }
                                }}
                            </small>
                        </div>
                    </div>
                </div>

                // Memo size indicator (real-time)
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
                                <span class="size-range">"(Required: 69-800 bytes)"</span>
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
                            <div class="error-message">{message}</div>
                        }.into_view()
                    } else {
                        view! { <div></div> }.into_view()
                    }
                }}

                // Submit button
                <div class="button-group">
                    <button
                        type="submit"
                        class="create-post-btn"
                        prop:disabled=move || {
                            is_submitting.get() ||
                            title.get().trim().is_empty() ||
                            title.get().len() > 128 ||
                            content.get().trim().is_empty() ||
                            content.get().len() > 512 ||
                            burn_amount.get() < 1 ||
                            session.with(|s| s.get_token_balance()) < burn_amount.get() as f64 ||
                            !calculate_memo_size().1
                        }
                    >
                        <i class="fas fa-paper-plane"></i>
                        {move || {
                            if is_submitting.get() {
                                "Creating Post...".to_string()
                            } else {
                                format!("Create Post (Burn {} MEMO)", burn_amount.get())
                            }
                        }}
                    </button>
                </div>
            </form>
        </div>
    }
}

/// Reply type enum
#[derive(Clone, Copy, PartialEq, Debug)]
enum ReplyType {
    Burn,
    Mint,
}

/// Reply Post Form component - reference DevlogForm design with two-column layout
#[component]
fn ReplyPostForm(
    session: RwSignal<Session>,
    post_id: u64,
    #[prop(default = "mint".to_string())] initial_type: String,
    on_close: Rc<dyn Fn()>,
    on_success: Rc<dyn Fn(String)>,
) -> impl IntoView {
    let on_close_signal = create_rw_signal(Some(on_close));
    let on_success_signal = create_rw_signal(Some(on_success));
    
    // Reply type state - can be toggled
    let initial_reply_type = if initial_type == "mint" { ReplyType::Mint } else { ReplyType::Burn };
    let (reply_type, set_reply_type) = create_signal(initial_reply_type);
    
    // Form state
    let (reply_title, set_reply_title) = create_signal(String::new());
    let (reply_content, set_reply_content) = create_signal(String::new());
    let (burn_amount, set_burn_amount) = create_signal(1u64);
    let (is_submitting, set_is_submitting) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    let (show_copied, set_show_copied) = create_signal(false);
    
    // Pixel art state
    let (pixel_art, set_pixel_art) = create_signal(Pixel::new_with_size(16));
    let (grid_size, set_grid_size) = create_signal(16usize);
    
    // Get token balance
    let token_balance = move || session.with(|s| s.get_token_balance());
    
    // Get user pubkey for memo calculation
    let get_user_pubkey = move || -> String {
        session.with(|s| s.get_public_key().unwrap_or_default())
    };
    
    // Calculate memo size in real time (69-800 bytes)
    let calculate_memo_size = move || -> (usize, bool, String) {
        use crate::core::rpc_forum::ForumConfig;
        
        let user_val = get_user_pubkey();
        let message_val = reply_content.get();
        let burn_val = if reply_type.get() == ReplyType::Burn { 
            burn_amount.get() * 1_000_000 
        } else { 
            0 
        };
        
        // For reply, we combine title + content + image as the message
        let full_message = if !reply_title.get().is_empty() || !pixel_art.get().to_optimal_string().is_empty() {
            let image_str = pixel_art.get().to_optimal_string();
            if image_str.is_empty() || image_str == "n:16" {
                format!("[{}] {}", reply_title.get(), message_val)
            } else {
                format!("[{}] {} [img:{}]", reply_title.get(), message_val, image_str)
            }
        } else {
            message_val.clone()
        };
        
        // Estimate memo size
        let estimated_size = if reply_type.get() == ReplyType::Burn {
            ForumConfig::estimate_burn_for_post_memo_size(&user_val, post_id, &full_message, burn_val)
        } else {
            ForumConfig::estimate_mint_for_post_memo_size(&user_val, post_id, &full_message)
        };
        
        let is_valid = estimated_size >= 69 && estimated_size <= 800;
        let status = if estimated_size < 69 {
            "Too short".to_string()
        } else if estimated_size > 800 {
            "Too long".to_string()
        } else {
            "Valid".to_string()
        };
        
        (estimated_size, is_valid, status)
    };
    
    let handle_close = move |_: web_sys::MouseEvent| {
        on_close_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };
    
    let handle_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        
        let title_val = reply_title.get();
        let content_val = reply_content.get();
        let burn_val = burn_amount.get();
        let current_reply_type = reply_type.get();
        let is_burn = current_reply_type == ReplyType::Burn;
        let image_str = pixel_art.get().to_optimal_string();
        
        // Build full message with title and image if present
        let full_message = if !title_val.is_empty() || (!image_str.is_empty() && image_str != "n:16") {
            if image_str.is_empty() || image_str == "n:16" {
                format!("[{}] {}", title_val, content_val)
            } else {
                format!("[{}] {} [img:{}]", title_val, content_val, image_str)
            }
        } else {
            content_val.clone()
        };
        
        // Validation for burn
        if is_burn && burn_val < 1 {
            set_error_message.set("Burn amount must be at least 1 MEMO".to_string());
            return;
        }
        
        if is_burn && token_balance() < burn_val as f64 {
            set_error_message.set("Insufficient MEMO balance".to_string());
            return;
        }
        
        // Check memo size
        let (_, is_valid, _) = calculate_memo_size();
        if !is_valid {
            set_error_message.set("Memo size is invalid (must be 69-800 bytes)".to_string());
            return;
        }
        
        set_is_submitting.set(true);
        set_error_message.set(String::new());
        
        let burn_lamports = burn_val * 1_000_000;
        let session_clone = session;
        let on_success_signal = on_success_signal.clone();
        
        spawn_local(async move {
            let mut session_update = session_clone.get_untracked();
            
            let result = if is_burn {
                session_update.burn_for_forum_post(post_id, burn_lamports, &full_message).await
            } else {
                session_update.mint_for_forum_post(post_id, &full_message).await
            };
            
            match result {
                Ok(signature) => {
                    log::info!("Reply posted: {}", signature);
                    
                    session_clone.update(|s| {
                        s.mark_balance_update_needed();
                    });
                    
                    on_success_signal.with_untracked(|cb_opt| {
                        if let Some(callback) = cb_opt.as_ref() {
                            callback(signature);
                        }
                    });
                },
                Err(e) => {
                    log::error!("Failed to post reply: {}", e);
                    set_error_message.set(format!("Failed to post reply: {}", e));
                    set_is_submitting.set(false);
                }
            }
        });
    };
    
    // Copy pixel art string
    let copy_string = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        let art_string = pixel_art.get().to_optimal_string();
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(&art_string);
            set_show_copied.set(true);
            spawn_local(async move {
                TimeoutFuture::new(2000).await;
                set_show_copied.set(false);
            });
        }
    };
    
    // Handle image import
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
                let current_grid_size = grid_size_signal.get();
                
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

    view! {
        <div class="reply-form">
            <div class="form-header">
                <h3 class="form-title">
                    <i class="fas fa-reply"></i>
                    "Reply to Post"
                </h3>
                <button 
                    type="button"
                    class="form-close-btn"
                    on:click=handle_close
                    prop:disabled=move || is_submitting.get()
                    title="Close"
                >
                    <i class="fas fa-times"></i>
                </button>
            </div>
            
            <form class="project-form" on:submit=handle_submit>
                <div class="form-layout">
                    // Left side: Title and Content
                    <div class="form-left">
                        // Reply Title
                        <div class="form-group">
                            <label for="reply-title">
                                <i class="fas fa-heading"></i>
                                "Reply Title (optional)"
                            </label>
                            <input
                                type="text"
                                id="reply-title"
                                prop:value=reply_title
                                on:input=move |ev| set_reply_title.set(event_target_value(&ev))
                                placeholder="Enter reply title (max 64 characters)..."
                                maxlength="64"
                                prop:disabled=move || is_submitting.get()
                            />
                            <small class="char-count">
                                {move || format!("{}/64 characters", reply_title.get().len())}
                            </small>
                        </div>

                        // Reply Content
                        <div class="form-group">
                            <label for="reply-content">
                                <i class="fas fa-align-left"></i>
                                "Content"
                            </label>
                            <textarea
                                id="reply-content"
                                prop:value=reply_content
                                on:input=move |ev| set_reply_content.set(event_target_value(&ev))
                                placeholder="Write your reply here (max 400 characters)..."
                                maxlength="400"
                                rows="6"
                                prop:disabled=move || is_submitting.get()
                            ></textarea>
                            <small class="char-count">
                                {move || format!("{}/400 characters", reply_content.get().len())}
                            </small>
                        </div>
                    </div>

                    // Right side: Image and Burn Amount
                    <div class="form-right">
                        <div class="pixel-art-editor">
                            <div class="pixel-art-header">
                                <label>
                                    <i class="fas fa-image"></i>
                                    "Reply Image (Optional)"
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
                                        prop:disabled=move || is_submitting.get()
                                    >
                                        <option value="16">"16×16 pixels"</option>
                                        <option value="32">"32×32 pixels"</option>
                                    </select>
                                    <button 
                                        type="button"
                                        class="import-btn"
                                        on:click=handle_import
                                        prop:disabled=move || is_submitting.get()
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
                                        size=160
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
                                        "Encoded: "
                                    </span>
                                    <span class="value">
                                        {move || {
                                            let art_string = pixel_art.get().to_optimal_string();
                                            if art_string.len() <= 16 {
                                                art_string
                                            } else {
                                                format!("{}...{}", &art_string[..8], &art_string[art_string.len()-6..])
                                            }
                                        }}
                                    </span>
                                    <div class="copy-container">
                                        <button
                                            type="button"
                                            class="copy-button"
                                            on:click=copy_string
                                            title="Copy"
                                        >
                                            <i class="fas fa-copy"></i>
                                        </button>
                                        <div class="copy-tooltip" class:show=move || show_copied.get()>
                                            "Copied!"
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                        
                        // Burn Amount - only show when burn mode is selected
                        <Show when=move || reply_type.get() == ReplyType::Burn>
                            <div class="form-group" style="margin-top: 16px;">
                                <label for="reply-burn-amount">
                                    <i class="fas fa-fire"></i>
                                    "Burn Amount (MEMO)"
                                </label>
                                <input
                                    type="number"
                                    id="reply-burn-amount"
                                    prop:value=burn_amount
                                    on:input=move |ev| {
                                        if let Ok(value) = event_target_value(&ev).parse::<u64>() {
                                            set_burn_amount.set(value.max(1));
                                        }
                                    }
                                    min="1"
                                    prop:disabled=move || is_submitting.get()
                                />
                                <small class="form-hint">
                                    <i class="fas fa-wallet"></i>
                                    {move || {
                                        let balance = session.with(|s| s.get_token_balance());
                                        view! {
                                            "Minimum: 1 MEMO (Available: "
                                            <span class={if balance >= 1.0 { "balance-sufficient" } else { "balance-insufficient" }}>
                                                {format!("{:.2} MEMO", balance)}
                                            </span>
                                            ")"
                                        }
                                    }}
                                </small>
                            </div>
                        </Show>
                        
                        // Mint info - only show when mint mode is selected
                        <Show when=move || reply_type.get() == ReplyType::Mint>
                            <div class="form-info mint-info" style="margin-top: 16px;">
                                <i class="fas fa-coins"></i>
                                <span>"Mint reply will mint 1 MEMO token for you"</span>
                            </div>
                        </Show>
                    </div>
                </div>

                // Memo size indicator (real-time)
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
                                <span class="size-range">"(Required: 69-800 bytes)"</span>
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
                            <div class="error-message">{message}</div>
                        }.into_view()
                    } else {
                        view! { <div></div> }.into_view()
                    }
                }}

                // Submit button with mode toggle
                <div class="form-actions-with-toggle">
                    // Mode toggle
                    <div class="action-mode-toggle">
                        <button
                            type="button"
                            class="mode-btn"
                            class:active=move || reply_type.get() == ReplyType::Mint
                            on:click=move |_| set_reply_type.set(ReplyType::Mint)
                            title="Mint Mode"
                        >
                            <i class="fas fa-coins"></i>
                        </button>
                        <button
                            type="button"
                            class="mode-btn burn-mode"
                            class:active=move || reply_type.get() == ReplyType::Burn
                            on:click=move |_| set_reply_type.set(ReplyType::Burn)
                            title="Burn Mode"
                        >
                            <i class="fas fa-fire"></i>
                        </button>
                    </div>
                    <button
                        type="submit"
                        class="submit-btn"
                        class:burn-submit=move || reply_type.get() == ReplyType::Burn
                        class:mint-submit=move || reply_type.get() == ReplyType::Mint
                        prop:disabled=move || {
                            is_submitting.get() ||
                            !calculate_memo_size().1
                        }
                    >
                        {move || if is_submitting.get() {
                            "Posting Reply...".to_string()
                        } else if reply_type.get() == ReplyType::Burn {
                            format!("🔥 Burn {} MEMO", burn_amount.get())
                        } else {
                            "🪙 Mint Tokens".to_string()
                        }}
                    </button>
                </div>
            </form>
        </div>
    }
}
