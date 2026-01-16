use leptos::*;
use std::rc::Rc;
use crate::core::session::Session;
use crate::core::rpc_blog::{
    BlogContractTransaction, BlogOperationType, BlogOperationDetails, BlogInfo,
    BlogCreationData,
};
use crate::core::rpc_base::RpcConnection;
use crate::core::rpc_mint::MintConfig;
use crate::core::pixel::Pixel;
use wasm_bindgen_futures::spawn_local;
use crate::pages::pixel_view::{LazyPixelView, PixelView};
use gloo_timers::future::TimeoutFuture;
use leptos::web_sys::window;
use web_sys::{HtmlInputElement, FileReader, Event, ProgressEvent};
use wasm_bindgen::{closure::Closure, JsCast};
use js_sys::Uint8Array;

/// Post type for New Post dialog
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostType {
    Burn,
    Mint,
}

/// Helper function to format timestamp as relative time
fn format_relative_time(timestamp: i64) -> String {
    let now = js_sys::Date::now() / 1000.0;
    let diff = (now as i64) - timestamp;
    
    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        let mins = diff / 60;
        format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if diff < 86400 {
        let hours = diff / 3600;
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else {
        let days = diff / 86400;
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    }
}

/// Helper function to shorten an address (show first 4 and last 4 characters)
fn shorten_address(address: &str) -> String {
    if address.len() <= 8 {
        address.to_string()
    } else {
        format!("{}...{}", &address[..4], &address[address.len()-4..])
    }
}

/// Helper function to format burn amount for display
fn format_burn_amount(amount: u64) -> String {
    let amount_f = amount as f64 / 1_000_000.0;
    if amount_f >= 1_000_000.0 {
        format!("{:.2}M", amount_f / 1_000_000.0)
    } else if amount_f >= 1_000.0 {
        format!("{:.2}K", amount_f / 1_000.0)
    } else if amount_f >= 1.0 {
        format!("{:.2}", amount_f)
    } else {
        format!("{:.4}", amount_f)
    }
}

/// Parse post message - handles both JSON format and plain text
/// Uses custom JSON parsing (like devlog) to preserve newlines and handle control characters
/// Returns (title, content, image)
fn parse_post_message(message: &str) -> (String, String, String) {
    // Clean NULL bytes but preserve newlines and other whitespace
    let cleaned: String = message
        .chars()
        .filter(|c| *c != '\0')  // Only remove NULL bytes
        .collect();
    let cleaned = cleaned.trim();
    
    // Check if it looks like JSON
    if cleaned.starts_with('{') {
        // Use custom JSON field extraction (like devlog) to handle unescaped newlines
        let title = extract_json_field(cleaned, "title").unwrap_or_default();
        let content = extract_json_field(cleaned, "content")
            .or_else(|| extract_json_field(cleaned, "message"))
            .unwrap_or_default();
        let image = extract_json_field(cleaned, "image").unwrap_or_default();
        
        return (title, content, image);
    }
    
    // Plain text message - no title, content is the message, no image
    (String::new(), cleaned.to_string(), String::new())
}

/// Extract a field value from JSON string (handles unescaped newlines)
/// Same approach as devlog parsing in project_page.rs
fn extract_json_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", field);
    let start = json.find(&pattern)? + pattern.len();
    let remaining = &json[start..];
    
    // Find the closing quote, handling escaped quotes
    // Use byte offset instead of char index to avoid UTF-8 boundary issues
    let mut end_byte = 0;
    let mut escaped = false;
    for c in remaining.chars() {
        if escaped {
            escaped = false;
            end_byte += c.len_utf8();
            continue;
        }
        if c == '\\' {
            escaped = true;
            end_byte += c.len_utf8();
            continue;
        }
        if c == '"' {
            break;
        }
        end_byte += c.len_utf8();
    }
    
    let value = &remaining[..end_byte];
    // Unescape the string - handle escaped quotes and backslashes, and \n sequences
    Some(value
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\"))
}

/// Render a transaction card based on operation type
fn render_transaction_card(
    transaction: BlogContractTransaction,
    session: RwSignal<Session>,
) -> impl IntoView {
    let burn_amount_display = format_burn_amount(transaction.burn_amount);
    let time_display = format_relative_time(transaction.timestamp);
    
    match transaction.details {
        BlogOperationDetails::Create { creator, name, description, image } => {
            view! {
                <div class="transaction-card transaction-create">
                    <div class="transaction-header">
                        <div class="transaction-icon">
                            <i class="fas fa-plus-circle"></i>
                        </div>
                        <div class="transaction-time">{time_display}</div>
                    </div>
                    <div class="transaction-body">
                        <div class="blog-info-horizontal">
                            {if !image.is_empty() && (image.starts_with("c:") || image.starts_with("n:")) {
                                view! {
                                    <div class="blog-image">
                                        <LazyPixelView
                                            art={image}
                                            size=64
                                        />
                                    </div>
                                }.into_view()
                            } else {
                                view! {
                                    <div class="blog-image-placeholder">
                                        <i class="fas fa-image"></i>
                                    </div>
                                }.into_view()
                            }}
                            <div class="blog-content">
                                <h3 class="blog-name">{name}</h3>
                                {if !description.is_empty() {
                                    view! {
                                        <p class="blog-description">{description}</p>
                                    }.into_view()
                                } else {
                                    view! { <div></div> }.into_view()
                                }}
                                <div class="blog-meta">
                                    <span class="blog-creator">
                                        <i class="fas fa-user"></i>
                                        {shorten_address(&creator)}
                                    </span>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div class="transaction-footer">
                        <div class="burn-stat">
                            <i class="fas fa-fire"></i>
                            <span class="burn-amount">{burn_amount_display}</span>
                            <span class="burn-label">" MEMO Burned"</span>
                        </div>
                    </div>
                </div>
            }.into_view()
        },
        BlogOperationDetails::Update { creator, name, description, image } => {
            let name_display = name.unwrap_or_else(|| "Blog".to_string());
            let description_str = description.unwrap_or_default();
            let image_str = image.unwrap_or_default();
            let has_valid_image = !image_str.is_empty() && (image_str.starts_with("c:") || image_str.starts_with("n:"));
            view! {
                <div class="transaction-card transaction-update">
                    <div class="transaction-header">
                        <div class="transaction-icon">
                            <i class="fas fa-edit"></i>
                        </div>
                        <div class="transaction-time">{time_display}</div>
                    </div>
                    <div class="transaction-body">
                        <div class="blog-info-horizontal">
                            {if has_valid_image {
                                view! {
                                    <div class="blog-image">
                                        <LazyPixelView
                                            art={image_str}
                                            size=64
                                        />
                                    </div>
                                }.into_view()
                            } else {
                                view! {
                                    <div class="blog-image-placeholder">
                                        <i class="fas fa-image"></i>
                                    </div>
                                }.into_view()
                            }}
                            <div class="blog-content">
                                <h3 class="blog-name">{name_display}</h3>
                                {if !description_str.is_empty() {
                                    view! {
                                        <p class="blog-description">{description_str}</p>
                                    }.into_view()
                                } else {
                                    view! { <div></div> }.into_view()
                                }}
                                <div class="blog-meta">
                                    <span class="blog-creator">
                                        <i class="fas fa-user"></i>
                                        {shorten_address(&creator)}
                                    </span>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div class="transaction-footer">
                        <div class="burn-stat">
                            <i class="fas fa-fire"></i>
                            <span class="burn-amount">{burn_amount_display}</span>
                            <span class="burn-label">" MEMO Burned"</span>
                        </div>
                    </div>
                </div>
            }.into_view()
        },
        BlogOperationDetails::Burn { burner, message } => {
            // Parse message JSON
            let (post_title, post_content, post_image) = parse_post_message(&message);
            let has_post_image = !post_image.is_empty() && post_image != "n:";
            
            // Fetch blog info for display
            let (blog_info, set_blog_info) = create_signal(None::<(String, String)>);
            let burner_clone = burner.clone();
            let burner_display = shorten_address(&burner);
            
            {
                let session_clone = session;
                create_effect(move |_| {
                    let burner_for_effect = burner_clone.clone();
                    spawn_local(async move {
                        let session_read = session_clone.get_untracked();
                        if let Ok(info) = session_read.get_user_blog(&burner_for_effect).await {
                            set_blog_info.set(Some((
                                info.name.clone(),
                                info.image.clone(),
                            )));
                        }
                    });
                });
            }
            
            view! {
                <div class="transaction-card transaction-burn">
                    <div class="transaction-header">
                        <div class="transaction-icon">
                            <i class="fas fa-fire"></i>
                        </div>
                        <div class="transaction-time">{time_display}</div>
                    </div>
                    <div class="transaction-body">
                        <div class="blog-info-horizontal">
                            {
                                // Show post image if available, otherwise show blog avatar
                                if has_post_image {
                                    view! {
                                        <div class="blog-image">
                                            <LazyPixelView
                                                art={post_image.clone()}
                                                size=64
                                            />
                                        </div>
                                    }.into_view()
                                } else {
                                    view! {
                                        {move || {
                                            let img = if let Some((_, blog_image)) = blog_info.get() {
                                                blog_image
                                            } else {
                                                String::new()
                                            };
                                            
                                            if !img.is_empty() && (img.starts_with("c:") || img.starts_with("n:")) {
                                                view! {
                                                    <div class="blog-image">
                                                        <LazyPixelView
                                                            art={img}
                                                            size=64
                                                        />
                                                    </div>
                                                }.into_view()
                                            } else {
                                                view! {
                                                    <div class="blog-image-placeholder">
                                                        <i class="fas fa-image"></i>
                                                    </div>
                                                }.into_view()
                                            }
                                        }}
                                    }.into_view()
                                }
                            }
                            <div class="blog-content">
                                // Post title or blog name
                                <h3 class="blog-name">
                                    {if !post_title.is_empty() {
                                        post_title.clone()
                                    } else {
                                        "".to_string()
                                    }}
                                </h3>
                                // Post content
                                {if !post_content.is_empty() {
                                    view! {
                                        <p class="blog-description">{post_content.clone()}</p>
                                    }.into_view()
                                } else {
                                    view! { <div></div> }.into_view()
                                }}
                                <div class="blog-meta">
                                    <span class="blog-creator">
                                        <i class="fas fa-user"></i>
                                        {burner_display}
                                    </span>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div class="transaction-footer">
                        <div class="burn-stat">
                            <i class="fas fa-fire"></i>
                            <span class="burn-amount">{burn_amount_display}</span>
                            <span class="burn-label">" MEMO Burned"</span>
                        </div>
                    </div>
                </div>
            }.into_view()
        },
        BlogOperationDetails::Mint { minter, message } => {
            // Parse message JSON
            let (post_title, post_content, post_image) = parse_post_message(&message);
            let has_post_image = !post_image.is_empty() && post_image != "n:";
            
            // Fetch blog info for display
            let (blog_info, set_blog_info) = create_signal(None::<(String, String)>);
            // Fetch current mint reward based on supply
            let (mint_reward, set_mint_reward) = create_signal(None::<f64>);
            let minter_clone = minter.clone();
            let minter_display = shorten_address(&minter);
            
            {
                let session_clone = session;
                create_effect(move |_| {
                    let minter_for_effect = minter_clone.clone();
                    spawn_local(async move {
                        let session_read = session_clone.get_untracked();
                        if let Ok(info) = session_read.get_user_blog(&minter_for_effect).await {
                            set_blog_info.set(Some((
                                info.name.clone(),
                                info.image.clone(),
                            )));
                        }
                        
                        // Fetch current supply and calculate mint reward
                        let rpc = RpcConnection::new();
                        if let Ok((supply, _tier)) = rpc.get_current_supply_tier_info().await {
                            let reward = MintConfig::calculate_mint_reward(supply);
                            set_mint_reward.set(Some(reward));
                        }
                    });
                });
            }
            
            view! {
                <div class="transaction-card transaction-mint">
                    <div class="transaction-header">
                        <div class="transaction-icon">
                            <i class="fas fa-coins"></i>
                        </div>
                        <div class="transaction-time">{time_display}</div>
                    </div>
                    <div class="transaction-body">
                        <div class="blog-info-horizontal">
                            {
                                // Show post image if available, otherwise show blog avatar
                                if has_post_image {
                                    view! {
                                        <div class="blog-image">
                                            <LazyPixelView
                                                art={post_image.clone()}
                                                size=64
                                            />
                                        </div>
                                    }.into_view()
                                } else {
                                    view! {
                                        {move || {
                                            let img = if let Some((_, blog_image)) = blog_info.get() {
                                                blog_image
                                            } else {
                                                String::new()
                                            };
                                            
                                            if !img.is_empty() && (img.starts_with("c:") || img.starts_with("n:")) {
                                                view! {
                                                    <div class="blog-image">
                                                        <LazyPixelView
                                                            art={img}
                                                            size=64
                                                        />
                                                    </div>
                                                }.into_view()
                                            } else {
                                                view! {
                                                    <div class="blog-image-placeholder">
                                                        <i class="fas fa-image"></i>
                                                    </div>
                                                }.into_view()
                                            }
                                        }}
                                    }.into_view()
                                }
                            }
                            <div class="blog-content">
                                // Post title or blog name
                                <h3 class="blog-name">
                                    {if !post_title.is_empty() {
                                        post_title.clone()
                                    } else {
                                        "".to_string()
                                    }}
                                </h3>
                                // Post content
                                {if !post_content.is_empty() {
                                    view! {
                                        <p class="blog-description">{post_content.clone()}</p>
                                    }.into_view()
                                } else {
                                    view! { <div></div> }.into_view()
                                }}
                                <div class="blog-meta">
                                    <span class="blog-creator">
                                        <i class="fas fa-user"></i>
                                        {minter_display}
                                    </span>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div class="transaction-footer">
                        <div class="mint-stat">
                            <i class="fas fa-coins"></i>
                            <span class="mint-amount">
                                {move || {
                                    if let Some(reward) = mint_reward.get() {
                                        MintConfig::format_mint_reward(reward)
                                    } else {
                                        "+? MEMO".to_string()
                                    }
                                }}
                            </span>
                            <span class="mint-label">" Minted"</span>
                        </div>
                    </div>
                </div>
            }.into_view()
        },
    }
}

/// Render featured activity card (all operations with burn amount > 0)
/// Layout follows memo project's featured card design
fn render_featured_card(
    transaction: BlogContractTransaction,
    session: RwSignal<Session>,
) -> impl IntoView {
    let burn_amount_display = format_burn_amount(transaction.burn_amount);
    let time_display = format_relative_time(transaction.timestamp);
    
    // Render different cards based on operation type
    match transaction.details {
        BlogOperationDetails::Burn { burner, message } => {
            // Parse message JSON
            let (post_title, post_content, post_image) = parse_post_message(&message);
            let has_post_image = !post_image.is_empty() && post_image != "n:";
            let burner_display = shorten_address(&burner);
            let burner_clone = burner.clone();
            
            // Fetch blog name for display
            let (blog_name, set_blog_name) = create_signal("Blog".to_string());
            {
                let session_clone = session;
                create_effect(move |_| {
                    let burner_for_effect = burner_clone.clone();
                    spawn_local(async move {
                        let session_read = session_clone.get_untracked();
                        if let Ok(info) = session_read.get_user_blog(&burner_for_effect).await {
                            set_blog_name.set(info.name.clone());
                        }
                    });
                });
            }
            
            view! {
                <div class="featured-card">
                    <div class="featured-card-content featured-burn">
                        <div class="featured-badge burn-badge">
                            <i class="fas fa-fire"></i>
                            "Blog Post"
                        </div>
                        
                        // Main content - horizontal layout like devlog
                        <div class="featured-post">
                            <div class="post-layout-horizontal">
                                // Image on the left
                                {if has_post_image {
                                    view! {
                                        <div class="post-image">
                                            <LazyPixelView
                                                art={post_image}
                                                size=100
                                            />
                                        </div>
                                    }.into_view()
                                } else {
                                    view! { <div></div> }.into_view()
                                }}
                                
                                // Content on the right
                                <div class="post-content-wrapper">
                                    {if !post_title.is_empty() {
                                        view! {
                                            <h3 class="post-title">{post_title}</h3>
                                        }.into_view()
                                    } else {
                                        view! { <div></div> }.into_view()
                                    }}
                                    
                                    {if !post_content.is_empty() {
                                        view! {
                                            <p class="post-content">{post_content}</p>
                                        }.into_view()
                                    } else {
                                        view! { <div></div> }.into_view()
                                    }}
                                    
                                    <div class="post-blog-info">
                                        <span>"Blog: "</span>
                                        <strong>{move || blog_name.get()}</strong>
                                    </div>
                                </div>
                            </div>
                        </div>
                        
                        // Stats at bottom
                        <div class="featured-stats">
                            <div class="featured-stat">
                                <i class="fas fa-fire"></i>
                                <span class="stat-value">{burn_amount_display}</span>
                            </div>
                            <div class="featured-stat">
                                <i class="fas fa-user"></i>
                                <span class="stat-value">{burner_display}</span>
                            </div>
                            <div class="featured-stat">
                                <i class="fas fa-clock"></i>
                                <span class="stat-value">{time_display}</span>
                            </div>
                        </div>
                    </div>
                </div>
            }.into_view()
        },
        BlogOperationDetails::Create { creator, name, description, image } => {
            let creator_display = shorten_address(&creator);
            let has_image = !image.is_empty() && (image.starts_with("c:") || image.starts_with("n:"));
            
            view! {
                <div class="featured-card">
                    <div class="featured-card-content featured-create">
                        <div class="featured-badge create-badge">
                            <i class="fas fa-blog"></i>
                            "New Blog"
                        </div>
                        
                        // Main content - horizontal layout
                        <div class="featured-post">
                            <div class="post-layout-horizontal">
                                // Image on the left
                                {if has_image {
                                    view! {
                                        <div class="post-image">
                                            <LazyPixelView
                                                art={image}
                                                size=100
                                            />
                                        </div>
                                    }.into_view()
                                } else {
                                    view! {
                                        <div class="post-image-placeholder">
                                            <i class="fas fa-blog"></i>
                                        </div>
                                    }.into_view()
                                }}
                                
                                // Content on the right
                                <div class="post-content-wrapper">
                                    <h3 class="post-title">{name}</h3>
                                    
                                    {if !description.is_empty() {
                                        view! {
                                            <p class="post-content">{description}</p>
                                        }.into_view()
                                    } else {
                                        view! { <div></div> }.into_view()
                                    }}
                                </div>
                            </div>
                        </div>
                        
                        // Stats at bottom
                        <div class="featured-stats">
                            <div class="featured-stat">
                                <i class="fas fa-fire"></i>
                                <span class="stat-value">{burn_amount_display}</span>
                            </div>
                            <div class="featured-stat">
                                <i class="fas fa-user"></i>
                                <span class="stat-value">{creator_display}</span>
                            </div>
                            <div class="featured-stat">
                                <i class="fas fa-clock"></i>
                                <span class="stat-value">{time_display}</span>
                            </div>
                        </div>
                    </div>
                </div>
            }.into_view()
        },
        BlogOperationDetails::Update { creator, name, description, image } => {
            let creator_display = shorten_address(&creator);
            let name_display = name.unwrap_or_else(|| "Blog".to_string());
            let description_str = description.unwrap_or_default();
            let image_str = image.unwrap_or_default();
            let has_valid_image = !image_str.is_empty() && (image_str.starts_with("c:") || image_str.starts_with("n:"));
            
            view! {
                <div class="featured-card">
                    <div class="featured-card-content featured-update">
                        <div class="featured-badge update-badge">
                            <i class="fas fa-edit"></i>
                            "Blog Updated"
                        </div>
                        
                        // Main content - horizontal layout
                        <div class="featured-post">
                            <div class="post-layout-horizontal">
                                // Image on the left
                                {if has_valid_image {
                                    view! {
                                        <div class="post-image">
                                            <LazyPixelView
                                                art={image_str}
                                                size=100
                                            />
                                        </div>
                                    }.into_view()
                                } else {
                                    view! {
                                        <div class="post-image-placeholder">
                                            <i class="fas fa-edit"></i>
                                        </div>
                                    }.into_view()
                                }}
                                
                                // Content on the right
                                <div class="post-content-wrapper">
                                    <h3 class="post-title">{name_display}</h3>
                                    
                                    {if !description_str.is_empty() {
                                        view! {
                                            <p class="post-content">{description_str}</p>
                                        }.into_view()
                                    } else {
                                        view! { <div></div> }.into_view()
                                    }}
                                </div>
                            </div>
                        </div>
                        
                        // Stats at bottom
                        <div class="featured-stats">
                            <div class="featured-stat">
                                <i class="fas fa-fire"></i>
                                <span class="stat-value">{burn_amount_display}</span>
                            </div>
                            <div class="featured-stat">
                                <i class="fas fa-user"></i>
                                <span class="stat-value">{creator_display}</span>
                            </div>
                            <div class="featured-stat">
                                <i class="fas fa-clock"></i>
                                <span class="stat-value">{time_display}</span>
                            </div>
                        </div>
                    </div>
                </div>
            }.into_view()
        },
        BlogOperationDetails::Mint { minter: _, message: _ } => {
            // Mint operations should not appear in featured (burn_amount = 0)
            // but handle the case anyway
            view! { <div class="featured-card-empty"></div> }.into_view()
        },
    }
}

/// Blog page component
#[component]
pub fn BlogPage(
    session: RwSignal<Session>,
) -> impl IntoView {
    let (transactions, set_transactions) = create_signal::<Vec<BlogContractTransaction>>(vec![]);
    let (featured_burns, set_featured_burns) = create_signal::<Vec<BlogContractTransaction>>(vec![]);
    let (loading, set_loading) = create_signal(true);
    let (error_message, set_error_message) = create_signal::<Option<String>>(None);
    let (current_featured_index, set_current_featured_index) = create_signal(0_usize);
    
    // Dialog states
    let (show_new_post_dialog, set_show_new_post_dialog) = create_signal(false);
    let (show_my_blog_view, set_show_my_blog_view) = create_signal(false);
    let (show_create_blog_dialog, set_show_create_blog_dialog) = create_signal(false);
    let (show_update_blog_dialog, set_show_update_blog_dialog) = create_signal(false);
    
    // User's blog info
    let (user_blog, set_user_blog) = create_signal::<Option<BlogInfo>>(None);
    let (user_blog_loading, set_user_blog_loading) = create_signal(false);
    
    // Load recent transactions
    let load_transactions = create_action(move |_: &()| {
        async move {
            set_loading.set(true);
            set_error_message.set(None);
            
            let rpc = RpcConnection::new();
            match rpc.get_recent_blog_contract_transactions().await {
                Ok(response) => {
                    log::info!("Loaded {} blog transactions", response.transactions.len());
                    
                    // Filter all operations with burn amount > 0 (create, update, burn)
                    // and sort by timestamp (newest first) for featured section
                    let mut burn_transactions: Vec<BlogContractTransaction> = response.transactions.iter()
                        .filter(|tx| tx.burn_amount > 0)
                        .cloned()
                        .collect();
                    
                    // Sort by timestamp (newest first) to show latest activity
                    burn_transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                    
                    // Take top 3 for featured section
                    let featured: Vec<BlogContractTransaction> = burn_transactions.iter()
                        .take(3)
                        .cloned()
                        .collect();
                    
                    set_featured_burns.set(featured);
                    set_transactions.set(response.transactions);
                },
                Err(e) => {
                    log::error!("Failed to load blog transactions: {}", e);
                    set_error_message.set(Some(format!("Failed to load transactions: {}", e)));
                }
            }
            
            set_loading.set(false);
        }
    });
    
    // Load transactions on mount
    create_effect(move |_| {
        load_transactions.dispatch(());
    });
    
    // Auto-rotate featured cards every 30 seconds
    {
        let interval_handle = set_interval_with_handle(
            move || {
                let featured = featured_burns.get();
                if featured.len() > 1 {
                    set_current_featured_index.update(|idx| {
                        *idx = (*idx + 1) % featured.len();
                    });
                }
            },
            std::time::Duration::from_secs(30),
        );
        
        on_cleanup(move || {
            if let Ok(handle) = interval_handle {
                handle.clear();
            }
        });
    }
    
    // Callback for when post is successful
    let on_post_success = {
        let set_show_new_post = set_show_new_post_dialog;
        Rc::new(move || {
            set_show_new_post.set(false);
            // Refresh transactions immediately
            load_transactions.dispatch(());
            // Also refresh after delay to catch confirmed transaction
            spawn_local(async move {
                TimeoutFuture::new(3_000).await;
                load_transactions.dispatch(());
            });
        })
    };
    
    // Callback for when blog is created
    let on_create_blog_success = {
        let set_show_create = set_show_create_blog_dialog;
        Rc::new(move |_signature: String| {
            set_show_create.set(false);
            // Refresh to show new blog immediately
            load_transactions.dispatch(());
            // Also refresh after delay to catch confirmed transaction
            spawn_local(async move {
                TimeoutFuture::new(3_000).await;
                load_transactions.dispatch(());
            });
        })
    };
    
    // Callback for when blog is updated
    let on_update_blog_success = {
        let set_show_update = set_show_update_blog_dialog;
        Rc::new(move |_signature: String| {
            set_show_update.set(false);
            // Refresh to show updated blog immediately
            load_transactions.dispatch(());
            // Also refresh after delay to catch confirmed transaction
            spawn_local(async move {
                TimeoutFuture::new(3_000).await;
                load_transactions.dispatch(());
            });
        })
    };
    
    view! {
        <div class="blog-page">
            // Action Bar with buttons
            <div class="blog-action-bar">
                <button
                    class="blog-action-btn create-blog-action-btn"
                    on:click=move |_| set_show_create_blog_dialog.set(true)
                    disabled=move || session.with(|s| s.get_public_key().is_err())
                >
                    <i class="fas fa-blog"></i>
                    "Create Blog"
                </button>
                
                <button
                    class="blog-action-btn new-post-btn"
                    on:click=move |_| set_show_new_post_dialog.set(true)
                    disabled=move || session.with(|s| s.get_public_key().is_err())
                >
                    <i class="fas fa-plus"></i>
                    "New Post"
                </button>
                
                <button
                    class="blog-action-btn my-blog-btn"
                    on:click=move |_| set_show_my_blog_view.set(true)
                    disabled=move || session.with(|s| s.get_public_key().is_err())
                >
                    <i class="fas fa-user"></i>
                    "My Blog"
                </button>
                
                <button
                    class="blog-action-btn refresh-btn"
                    on:click=move |_| load_transactions.dispatch(())
                    disabled=move || loading.get()
                    title="Refresh transactions"
                >
                    <i class="fas fa-sync-alt" class:fa-spin=move || loading.get()></i>
                    "Refresh"
                </button>
            </div>
            
            // Featured Activity Section (with 3D carousel effect)
            <Show when=move || !featured_burns.get().is_empty()>
                <div class="blog-featured-section">
                    <h2 class="section-title">
                        <i class="fas fa-star"></i>
                        "Featured Activity"
                    </h2>
                    <div class="blog-carousel-container">
                        <div class="blog-carousel-track">
                            {move || {
                                let featured = featured_burns.get();
                                let idx = current_featured_index.get();
                                
                                if featured.is_empty() {
                                    return view! { <div class="empty-featured"></div> }.into_view();
                                }
                                
                                let len = featured.len();
                                let prev_idx = if idx == 0 { len - 1 } else { idx - 1 };
                                let next_idx = (idx + 1) % len;
                                
                                view! {
                                    // Back card (prev) - clickable
                                    <div 
                                        class="blog-featured-card blog-featured-card-back"
                                        on:click=move |_| {
                                            set_current_featured_index.set(prev_idx);
                                        }
                                    >
                                        {render_featured_card(featured[prev_idx].clone(), session)}
                                    </div>
                                    
                                    // Front card (current) - not clickable
                                    <div class="blog-featured-card blog-featured-card-front">
                                        {render_featured_card(featured[idx].clone(), session)}
                                    </div>
                                    
                                    // Forward card (next) - clickable
                                    <div 
                                        class="blog-featured-card blog-featured-card-next"
                                        on:click=move |_| {
                                            set_current_featured_index.set(next_idx);
                                        }
                                    >
                                        {render_featured_card(featured[next_idx].clone(), session)}
                                    </div>
                                }.into_view()
                            }}
                        </div>
                    </div>
                    
                    // Carousel indicators
                    <Show when=move || { featured_burns.get().len() > 1 }>
                        <div class="blog-carousel-indicators">
                            {move || {
                                let featured = featured_burns.get();
                                let idx = current_featured_index.get();
                                (0..featured.len()).map(|i| {
                                    view! {
                                        <div 
                                            class="blog-indicator"
                                            class:active=move || idx == i
                                            on:click=move |_| set_current_featured_index.set(i)
                                        ></div>
                                    }
                                }).collect::<Vec<_>>()
                            }}
                        </div>
                    </Show>
                </div>
            </Show>
            
            // Recent Blogs Section
            <div class="transactions-section">
                <h2 class="section-title">
                    <i class="fas fa-history"></i>
                    "Recent Blogs"
                </h2>
                
                <Show
                    when=move || !loading.get()
                    fallback=move || view! {
                        <div class="loading-container">
                            <div class="loading-spinner"></div>
                            <p>"Loading transactions..."</p>
                        </div>
                    }
                >
                    <Show
                        when=move || error_message.get().is_none()
                        fallback=move || view! {
                            <div class="error-message">
                                <i class="fas fa-exclamation-triangle"></i>
                                {error_message.get().unwrap_or_default()}
                            </div>
                        }
                    >
                        <Show
                            when=move || !transactions.get().is_empty()
                            fallback=move || view! {
                                <div class="empty-state">
                                    <i class="fas fa-inbox"></i>
                                    <p>"No transactions found"</p>
                                </div>
                            }
                        >
                            <div class="transactions-list">
                                {move || {
                                    transactions.get().into_iter().map(|tx| {
                                        render_transaction_card(tx, session)
                                    }).collect::<Vec<_>>()
                                }}
                            </div>
                        </Show>
                    </Show>
                </Show>
            </div>
            
            // New Post Dialog
            <Show when=move || show_new_post_dialog.get()>
                <div class="modal-overlay">
                    <NewPostForm
                        session=session
                        on_close=Rc::new(move || set_show_new_post_dialog.set(false))
                        on_success=on_post_success.clone()
                        on_create_blog=Rc::new(move || {
                            set_show_new_post_dialog.set(false);
                            set_show_create_blog_dialog.set(true);
                        })
                    />
                </div>
            </Show>
            
            // My Blog View
            <Show when=move || show_my_blog_view.get()>
                <div class="modal-overlay my-blog-overlay">
                    <MyBlogView
                        session=session
                        on_close=Rc::new(move || set_show_my_blog_view.set(false))
                        on_create_blog=Rc::new(move || {
                            set_show_my_blog_view.set(false);
                            set_show_create_blog_dialog.set(true);
                        })
                        on_update_blog=Rc::new(move || {
                            set_show_my_blog_view.set(false);
                            set_show_update_blog_dialog.set(true);
                        })
                        on_new_post=Rc::new(move || {
                            set_show_my_blog_view.set(false);
                            set_show_new_post_dialog.set(true);
                        })
                    />
                </div>
            </Show>
            
            // Create Blog Dialog
            <Show when=move || show_create_blog_dialog.get()>
                <div class="modal-overlay">
                    <CreateBlogForm
                        session=session
                        on_close=Rc::new(move || set_show_create_blog_dialog.set(false))
                        on_success=on_create_blog_success.clone()
                    />
                </div>
            </Show>
            
            // Update Blog Dialog
            <Show when=move || show_update_blog_dialog.get()>
                <div class="modal-overlay">
                    <UpdateBlogForm
                        session=session
                        on_close=Rc::new(move || set_show_update_blog_dialog.set(false))
                        on_success=on_update_blog_success.clone()
                    />
                </div>
            </Show>
        </div>
    }
}

/// New Post Form - for creating burn or mint posts with pixel art support
/// Layout follows New Devlog dialog design
#[component]
fn NewPostForm(
    session: RwSignal<Session>,
    on_close: Rc<dyn Fn()>,
    on_success: Rc<dyn Fn()>,
    on_create_blog: Rc<dyn Fn()>,
) -> impl IntoView {
    let on_close_signal = create_rw_signal(Some(on_close));
    let on_success_signal = create_rw_signal(Some(on_success));
    let on_create_blog_signal = create_rw_signal(Some(on_create_blog));
    
    // Form state
    let (post_type, set_post_type) = create_signal(PostType::Mint); // Default to Mint
    let (post_title, set_post_title) = create_signal(String::new());
    let (message, set_message) = create_signal(String::new());
    let (burn_amount, set_burn_amount) = create_signal(1u64); // Minimum 1 MEMO for blog
    let (pixel_art, set_pixel_art) = create_signal(Pixel::new_with_size(16));
    let (grid_size, set_grid_size) = create_signal(16usize);
    let (is_posting, set_is_posting) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    let (user_has_blog, set_user_has_blog) = create_signal(false);
    let (loading_blog, set_loading_blog) = create_signal(true);
    let (show_copied, set_show_copied) = create_signal(false);
    
    // Get image data
    let get_image_data = move || -> String {
        pixel_art.get().to_optimal_string()
    };
    
    // Get burner pubkey
    let get_burner_pubkey = move || -> String {
        session.with(|s| s.get_public_key().unwrap_or_default())
    };
    
    // Build final message JSON (same format as devlog)
    // Format: {"type":"post","title":"...","content":"...","image":"..."}
    let build_final_message = move || -> String {
        let title = post_title.get().trim().to_string();
        let content = message.get().trim().to_string();
        let image = get_image_data();
        
        // Always include image field (consistent with devlog format)
        format!(r#"{{"type":"post","title":"{}","content":"{}","image":"{}"}}"#,
            title.replace('\\', "\\\\").replace('"', "\\\""),
            content.replace('\\', "\\\\").replace('"', "\\\""),
            image.replace('\\', "\\\\").replace('"', "\\\""))
    };
    
    // Calculate memo size in real time
    let calculate_memo_size = move || -> (usize, bool, String) {
        let final_message = build_final_message();
        
        // Estimate memo size (simplified calculation)
        // BurnMemo structure: category(4) + operation(~12) + burner(44) + message + amount(8) + base64 overhead
        let estimated_size = 80 + final_message.len() + (final_message.len() / 3);
        
        let is_valid = estimated_size >= 69 && estimated_size <= 800;
        let status = if is_valid {
            "✅ Valid".to_string()
        } else if estimated_size < 69 {
            "❌ Too short".to_string()
        } else {
            "❌ Too long".to_string()
        };
        (estimated_size, is_valid, status)
    };
    
    // Load user's blog - check if user has a blog
    create_effect(move |_| {
        spawn_local(async move {
            set_loading_blog.set(true);
            let session_read = session.get_untracked();
            if let Ok(pubkey) = session_read.get_public_key() {
                if let Ok(has_blog) = session_read.user_has_blog(&pubkey).await {
                    set_user_has_blog.set(has_blog);
                }
            }
            set_loading_blog.set(false);
        });
    });
    
    let handle_close = move |_| {
        on_close_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };
    
    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        
        if is_posting.get() {
            return;
        }
        
        if !user_has_blog.get() {
            set_error_message.set("❌ You need to create a blog first".to_string());
            return;
        }
        
        let title = post_title.get().trim().to_string();
        if title.is_empty() || title.len() > 64 {
            set_error_message.set(format!("❌ Title must be 1-64 characters, got {}", title.len()));
            return;
        }
        
        let content = message.get().trim().to_string();
        if content.len() > 500 {
            set_error_message.set(format!("❌ Content must be at most 500 characters, got {}", content.len()));
            return;
        }
        
        let final_message = build_final_message();
        
        // Check memo size
        let (memo_size, is_valid, _) = calculate_memo_size();
        if !is_valid {
            set_error_message.set(format!("❌ Memo size ({} bytes) must be between 69-800 bytes", memo_size));
            return;
        }
        
        let post_type_val = post_type.get();
        let amount = burn_amount.get();
        
        if post_type_val == PostType::Burn && amount < 1 {
            set_error_message.set("❌ Burn amount must be at least 1 MEMO".to_string());
            return;
        }
        
        // Check balance for burn
        if post_type_val == PostType::Burn {
            let token_balance = session.with_untracked(|s| s.get_token_balance());
            if token_balance < amount as f64 {
                set_error_message.set(format!("❌ Insufficient balance. Required: {} MEMO, Available: {:.2} MEMO", amount, token_balance));
                return;
            }
        }
        
        set_is_posting.set(true);
        set_error_message.set(String::new());
        
        spawn_local(async move {
            let result = if post_type_val == PostType::Burn {
                let mut session_update = session.get_untracked();
                session_update.burn_tokens_for_blog(amount, &final_message).await
            } else {
                let mut session_update = session.get_untracked();
                session_update.mint_tokens_for_blog(&final_message).await
            };
            
            match result {
                Ok(_signature) => {
                    session.update(|s| s.mark_balance_update_needed());
                    on_success_signal.with_untracked(|cb_opt| {
                        if let Some(callback) = cb_opt.as_ref() {
                            callback();
                        }
                    });
                },
                Err(e) => {
                    set_error_message.set(format!("❌ Failed to post: {}", e));
                    set_is_posting.set(false);
                }
            }
        });
    };
    
    // Handle image import
    let handle_import = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        
        let window_obj = web_sys::window().unwrap();
        let document = window_obj.document().unwrap();
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
    
    // Handle copy pixel art string
    let copy_string = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        
        let art_string = pixel_art.get().to_optimal_string();
        if let Some(window_obj) = window() {
            let clipboard = window_obj.navigator().clipboard();
            let _ = clipboard.write_text(&art_string);
            set_show_copied.set(true);
            
            spawn_local(async move {
                TimeoutFuture::new(3000).await;
                set_show_copied.set(false);
            });
        }
    };
    
    view! {
        <div class="new-post-form">
            <div class="form-header">
                <h3 class="form-title">
                    <i class="fas fa-edit"></i>
                    "New Post"
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
            
            <Show
                when=move || !loading_blog.get()
                fallback=|| view! {
                    <div class="form-loading">
                        <div class="loading-spinner"></div>
                        <p>"Loading your blog..."</p>
                    </div>
                }
            >
                <Show
                    when=move || user_has_blog.get()
                    fallback=move || {
                        let handle_create = move |_| {
                            on_create_blog_signal.with_untracked(|cb_opt| {
                                if let Some(callback) = cb_opt.as_ref() {
                                    callback();
                                }
                            });
                        };
                        view! {
                            <div class="no-blog-container">
                                <div class="no-blog-message">
                                    <i class="fas fa-plus-circle"></i>
                                    <h4>"You don't have a blog yet"</h4>
                                    <p>"Create your blog to start posting"</p>
                                </div>
                                <button
                                    class="create-blog-btn"
                                    on:click=handle_create
                                >
                                    <i class="fas fa-plus"></i>
                                    "Create Blog"
                                </button>
                            </div>
                        }
                    }
                >
                    <form class="blog-form" on:submit=handle_submit>
                        <div class="form-layout">
                            // Left side: Title and Content
                            <div class="form-left">
                                // Post Title
                                <div class="form-group">
                                    <label for="post-title">
                                        <i class="fas fa-heading"></i>
                                        "Post Title"
                                        <span class="required">*</span>
                                    </label>
                                    <input
                                        type="text"
                                        id="post-title"
                                        prop:value=post_title
                                        on:input=move |ev| set_post_title.set(event_target_value(&ev))
                                        placeholder="Enter post title (1-64 characters)..."
                                        maxlength="64"
                                        prop:disabled=move || is_posting.get()
                                    />
                                    <small class="char-count">
                                        {move || format!("{}/64 characters", post_title.get().len())}
                                    </small>
                                </div>
                                
                                // Post Content
                                <div class="form-group">
                                    <label for="post-content">
                                        <i class="fas fa-align-left"></i>
                                        "Content"
                                    </label>
                                    <textarea
                                        id="post-content"
                                        prop:value=message
                                        on:input=move |ev| set_message.set(event_target_value(&ev))
                                        placeholder="Write your post content here (max 500 characters)..."
                                        maxlength="500"
                                        rows="6"
                                        prop:disabled=move || is_posting.get()
                                    ></textarea>
                                    <small class="char-count">
                                        {move || format!("{}/500 characters", message.get().len())}
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
                                                prop:disabled=move || is_posting.get()
                                            >
                                                <option value="16">"16×16 pixels"</option>
                                                <option value="32">"32×32 pixels"</option>
                                            </select>
                                            <button
                                                type="button"
                                                class="import-btn"
                                                on:click=handle_import
                                                prop:disabled=move || is_posting.get()
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
                                
                                // Burn Amount (only for burn type)
                                <Show when=move || post_type.get() == PostType::Burn>
                                    <div class="form-group" style="margin-top: 16px;">
                                        <label for="burn-amount">
                                            <i class="fas fa-fire"></i>
                                            "Burn Amount (MEMO)"
                                        </label>
                                        <input
                                            type="number"
                                            id="burn-amount"
                                            prop:value=move || burn_amount.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(val) = event_target_value(&ev).parse::<u64>() {
                                                    set_burn_amount.set(val.max(1));
                                                }
                                            }
                                            min="1"
                                            prop:disabled=move || is_posting.get()
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
                            </div>
                        </div>
                        
                        // Memo size indicator
                        <div class="memo-size-indicator post">
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
                                            </div>
                                        </div>
                                    }
                                }}
                            </div>
                        </div>
                        
                        // Error message
                        <Show when=move || !error_message.get().is_empty()>
                            <div class="form-error">
                                {error_message}
                            </div>
                        </Show>
                        
                        // Submit button with mode toggle
                        <div class="form-actions">
                            // Mode toggle
                            <div class="action-mode-toggle">
                                <button
                                    type="button"
                                    class="mode-btn"
                                    class:active=move || post_type.get() == PostType::Mint
                                    on:click=move |_| set_post_type.set(PostType::Mint)
                                    title="Mint Mode"
                                >
                                    <i class="fas fa-coins"></i>
                                </button>
                                <button
                                    type="button"
                                    class="mode-btn burn-mode"
                                    class:active=move || post_type.get() == PostType::Burn
                                    on:click=move |_| set_post_type.set(PostType::Burn)
                                    title="Burn Mode"
                                >
                                    <i class="fas fa-fire"></i>
                                </button>
                            </div>
                            <button
                                type="submit"
                                class="submit-btn"
                                class:burn-submit=move || post_type.get() == PostType::Burn
                                class:mint-submit=move || post_type.get() == PostType::Mint
                                prop:disabled=move || is_posting.get()
                            >
                                {move || if is_posting.get() {
                                    "Posting...".to_string()
                                } else if post_type.get() == PostType::Burn {
                                    format!("🔥 Burn {} MEMO", burn_amount.get())
                                } else {
                                    "🪙 Mint Tokens".to_string()
                                }}
                            </button>
                        </div>
                    </form>
                </Show>
            </Show>
        </div>
    }
}

/// My Blog View - shows user's blog and posts
#[component]
fn MyBlogView(
    session: RwSignal<Session>,
    on_close: Rc<dyn Fn()>,
    on_create_blog: Rc<dyn Fn()>,
    on_update_blog: Rc<dyn Fn()>,
    on_new_post: Rc<dyn Fn()>,
) -> impl IntoView {
    let on_close_signal = create_rw_signal(Some(on_close));
    let on_create_signal = create_rw_signal(Some(on_create_blog));
    let on_update_signal = create_rw_signal(Some(on_update_blog));
    let on_new_post_signal = create_rw_signal(Some(on_new_post));
    
    let (user_blog, set_user_blog) = create_signal::<Option<BlogInfo>>(None);
    let (blog_posts, set_blog_posts) = create_signal::<Vec<BlogContractTransaction>>(vec![]);
    let (loading, set_loading) = create_signal(true);
    let (error_message, set_error_message) = create_signal::<Option<String>>(None);
    
    // Load user's blog and posts
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error_message.set(None);
            
            let session_read = session.get_untracked();
            if let Ok(pubkey) = session_read.get_public_key() {
                // Find user's blog (now directly by pubkey)
                if let Ok(blog) = session_read.get_user_blog(&pubkey).await {
                    set_user_blog.set(Some(blog.clone()));
                    
                    // Load recent posts for this blog
                    let rpc = RpcConnection::new();
                    if let Ok(response) = rpc.get_recent_blog_contract_transactions().await {
                        let user_posts: Vec<BlogContractTransaction> = response.transactions
                            .into_iter()
                            .filter(|tx| {
                                match &tx.details {
                                    BlogOperationDetails::Burn { burner, .. } |
                                    BlogOperationDetails::Mint { minter: burner, .. } => *burner == blog.creator,
                                    _ => false,
                                }
                            })
                            .take(10)
                            .collect();
                        set_blog_posts.set(user_posts);
                    }
                }
            } else {
                set_error_message.set(Some("Not logged in".to_string()));
            }
            
            set_loading.set(false);
        });
    });
    
    let handle_close = move |_| {
        on_close_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };
    
    let handle_create = move |_| {
        on_create_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };
    
    let handle_update = move |_| {
        on_update_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };
    
    let handle_new_post = move |_| {
        on_new_post_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };
    
    view! {
        <div class="my-blog-view">
            <div class="form-header">
                <h3 class="form-title">
                    <i class="fas fa-blog"></i>
                    "My Blog"
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
            
            <Show
                when=move || !loading.get()
                fallback=|| view! {
                    <div class="loading-container">
                        <div class="loading-spinner"></div>
                        <p>"Loading your blog..."</p>
                    </div>
                }
            >
                <Show
                    when=move || error_message.get().is_none()
                    fallback=move || view! {
                        <div class="error-message">
                            <i class="fas fa-exclamation-triangle"></i>
                            {error_message.get().unwrap_or_default()}
                        </div>
                    }
                >
                    <Show
                        when=move || user_blog.get().is_some()
                        fallback=move || view! {
                            <div class="no-blog-container">
                                <div class="no-blog-message">
                                    <i class="fas fa-plus-circle"></i>
                                    <h4>"You don't have a blog yet"</h4>
                                    <p>"Create your blog to start posting"</p>
                                </div>
                                <button
                                    class="create-blog-btn"
                                    on:click=handle_create
                                >
                                    <i class="fas fa-plus"></i>
                                    "Create Blog"
                                </button>
                            </div>
                        }
                    >
                        // Blog Info
                        {move || {
                            if let Some(blog) = user_blog.get() {
                                view! {
                                    <div class="blog-info-card">
                                        <div class="blog-header-row">
                                            {if !blog.image.is_empty() && (blog.image.starts_with("c:") || blog.image.starts_with("n:")) {
                                                view! {
                                                    <div class="blog-avatar">
                                                        <LazyPixelView
                                                            art={blog.image.clone()}
                                                            size=80
                                                        />
                                                    </div>
                                                }.into_view()
                                            } else {
                                                view! {
                                                    <div class="blog-avatar-placeholder">
                                                        <i class="fas fa-blog"></i>
                                                    </div>
                                                }.into_view()
                                            }}
                                            
                                            <div class="blog-meta">
                                                <h4 class="blog-name">{blog.name.clone()}</h4>
                                                <span class="blog-creator">{shorten_address(&blog.creator)}</span>
                                            </div>
                                        </div>
                                        
                                        {if !blog.description.is_empty() {
                                            view! {
                                                <p class="blog-description">{blog.description.clone()}</p>
                                            }.into_view()
                                        } else {
                                            view! { <div></div> }.into_view()
                                        }}
                                        
                                        <div class="blog-stats-row">
                                            <div class="blog-stat">
                                                <i class="fas fa-fire"></i>
                                                <span>{format_burn_amount(blog.burned_amount)}</span>
                                                <span class="stat-label">"Burned"</span>
                                            </div>
                                            <div class="blog-stat">
                                                <i class="fas fa-comment"></i>
                                                <span>{blog.memo_count}</span>
                                                <span class="stat-label">"Posts"</span>
                                            </div>
                                        </div>
                                        
                                        <div class="blog-actions">
                                            <button class="action-btn update-btn" on:click=handle_update>
                                                <i class="fas fa-edit"></i>
                                                "Update"
                                            </button>
                                            <button class="action-btn new-post-btn" on:click=handle_new_post>
                                                <i class="fas fa-plus"></i>
                                                "New Post"
                                            </button>
                                        </div>
                                    </div>
                                }.into_view()
                            } else {
                                view! { <div></div> }.into_view()
                            }
                        }}
                        
                        // Recent Posts
                        <div class="my-posts-section">
                            <h4 class="section-subtitle">
                                <i class="fas fa-history"></i>
                                "Recent Posts"
                            </h4>
                            
                            <Show
                                when=move || !blog_posts.get().is_empty()
                                fallback=|| view! {
                                    <div class="empty-posts">
                                        <i class="fas fa-inbox"></i>
                                        <p>"No posts yet"</p>
                                    </div>
                                }
                            >
                                <div class="posts-list">
                                    {move || {
                                        blog_posts.get().into_iter().map(|tx| {
                                            // Parse message content - handle both JSON and plain text
                                            let (title, content, image) = match &tx.details {
                                                BlogOperationDetails::Burn { message, .. } |
                                                BlogOperationDetails::Mint { message, .. } => {
                                                    parse_post_message(message)
                                                },
                                                _ => (String::new(), String::new(), String::new())
                                            };
                                            
                                            let has_image = !image.is_empty() && image != "n:";
                                            let image_clone = image.clone();
                                            
                                            view! {
                                                <div class="post-card">
                                                    <div class="post-header">
                                                        {if matches!(tx.operation_type, BlogOperationType::BurnForBlog) {
                                                            view! {
                                                                <span class="post-type burn">
                                                                    <i class="fas fa-fire"></i>
                                                                    "Burn"
                                                                </span>
                                                            }.into_view()
                                                        } else {
                                                            view! {
                                                                <span class="post-type mint">
                                                                    <i class="fas fa-coins"></i>
                                                                    "Mint"
                                                                </span>
                                                            }.into_view()
                                                        }}
                                                        <span class="post-time">
                                                            {format_relative_time(tx.timestamp)}
                                                        </span>
                                                    </div>
                                                    
                                                    // Post title
                                                    {if !title.is_empty() {
                                                        view! {
                                                            <h4 class="post-title">{title.clone()}</h4>
                                                        }.into_view()
                                                    } else {
                                                        view! { <div></div> }.into_view()
                                                    }}
                                                    
                                                    // Post content and image
                                                    <div class="post-body">
                                                        {if has_image {
                                                            view! {
                                                                <div class="post-image">
                                                                    <PixelView
                                                                        art=image_clone
                                                                        size=64
                                                                        editable=false
                                                                        show_grid=false
                                                                        on_click=Box::new(|_, _| {})
                                                                    />
                                                                </div>
                                                            }.into_view()
                                                        } else {
                                                            view! { <div></div> }.into_view()
                                                        }}
                                                        
                                                        {if !content.is_empty() {
                                                            view! {
                                                                <p class="post-message">{content.clone()}</p>
                                                            }.into_view()
                                                        } else {
                                                            view! { <div></div> }.into_view()
                                                        }}
                                                    </div>
                                                    
                                                    {if tx.burn_amount > 0 {
                                                        view! {
                                                            <div class="post-burn">
                                                                <i class="fas fa-fire"></i>
                                                                {format_burn_amount(tx.burn_amount)}
                                                                " MEMO"
                                                            </div>
                                                        }.into_view()
                                                    } else {
                                                        view! { <div></div> }.into_view()
                                                    }}
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()
                                    }}
                                </div>
                            </Show>
                        </div>
                    </Show>
                </Show>
            </Show>
        </div>
    }
}

/// Create Blog Form
#[component]
fn CreateBlogForm(
    session: RwSignal<Session>,
    on_close: Rc<dyn Fn()>,
    on_success: Rc<dyn Fn(String)>,
) -> impl IntoView {
    let on_close_signal = create_rw_signal(Some(on_close));
    let on_success_signal = create_rw_signal(Some(on_success));
    
    // Form state
    let (blog_name, set_blog_name) = create_signal(String::new());
    let (blog_description, set_blog_description) = create_signal(String::new());
    let (burn_amount, set_burn_amount) = create_signal(1u64); // Minimum 1 MEMO for blog creation
    let (pixel_art, set_pixel_art) = create_signal(Pixel::new_with_size(16));
    let (grid_size, set_grid_size) = create_signal(16usize);
    let (is_creating, set_is_creating) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    let (creating_status, set_creating_status) = create_signal(String::new());
    
    let get_image_data = move || -> String {
        pixel_art.get().to_optimal_string()
    };
    
    // Calculate memo size in real time (69-800 bytes)
    let calculate_memo_size = move || -> (usize, bool, String) {
        let name = blog_name.get().trim().to_string();
        let description = blog_description.get().trim().to_string();
        let image_data = get_image_data();
        let amount = burn_amount.get() * 1_000_000; // lamports
        
        // Use dummy creator pubkey for calculation
        let blog_data = BlogCreationData::new("11111111111111111111111111111111".to_string(), name, description, image_data);
        
        match blog_data.calculate_final_memo_size(amount) {
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
            Err(e) => (0, false, format!("❌ Error: {}", e)),
        }
    };
    
    let handle_close = move |_| {
        if is_creating.get() {
            return;
        }
        on_close_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };
    
    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        
        if is_creating.get() {
            return;
        }
        
        let name = blog_name.get().trim().to_string();
        if name.is_empty() || name.len() > 64 {
            set_error_message.set(format!("❌ Blog name must be 1-64 characters, got {}", name.len()));
            return;
        }
        
        let description = blog_description.get().trim().to_string();
        if description.len() > 256 {
            set_error_message.set(format!("❌ Description must be at most 256 characters, got {}", description.len()));
            return;
        }
        
        let amount = burn_amount.get();
        if amount < 1 {
            set_error_message.set("❌ Burn amount must be at least 1 MEMO".to_string());
            return;
        }
        
        // Check memo size
        let (memo_size, is_valid, _) = calculate_memo_size();
        if !is_valid {
            set_error_message.set(format!("❌ Memo size ({} bytes) must be between 69-800 bytes", memo_size));
            return;
        }
        
        // Check balance
        let token_balance = session.with_untracked(|s| s.get_token_balance());
        if token_balance < amount as f64 {
            set_error_message.set(format!("❌ Insufficient balance. Required: {} MEMO, Available: {:.2} MEMO", amount, token_balance));
            return;
        }
        
        let image = get_image_data();
        
        set_is_creating.set(true);
        set_error_message.set(String::new());
        set_creating_status.set("Creating blog...".to_string());
        
        spawn_local(async move {
            let mut session_update = session.get_untracked();
            
            match session_update.create_blog(&name, &description, &image, amount).await {
                Ok(signature) => {
                    set_creating_status.set("Blog created successfully!".to_string());
                    session.update(|s| s.mark_balance_update_needed());
                    
                    on_success_signal.with_untracked(|cb_opt| {
                        if let Some(callback) = cb_opt.as_ref() {
                            callback(signature);
                        }
                    });
                },
                Err(e) => {
                    set_error_message.set(format!("❌ Failed to create blog: {}", e));
                    set_is_creating.set(false);
                }
            }
        });
    };
    
    let handle_grid_size_change = move |ev: leptos::ev::Event| {
        if let Ok(size) = event_target_value(&ev).parse::<usize>() {
            if size == 16 || size == 32 {
                set_grid_size.set(size);
                set_pixel_art.set(Pixel::new_with_size(size));
            }
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
    
    // Copy string state
    let (show_copied, set_show_copied) = create_signal(false);
    
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
    
    view! {
        <div class="create-blog-form">
            <div class="form-header">
                <h3 class="form-title">
                    <i class="fas fa-blog"></i>
                    "Create Blog"
                </h3>
                <button
                    type="button"
                    class="form-close-btn"
                    on:click=handle_close
                    title="Close"
                    prop:disabled=move || is_creating.get()
                >
                    <i class="fas fa-times"></i>
                </button>
            </div>
            
            <form class="blog-form" on:submit=handle_submit>
                <div class="form-layout">
                    // Left side: Basic Information
                    <div class="form-left">
                        // Blog Name
                        <div class="form-group">
                            <label for="blog-name">
                                <i class="fas fa-pencil-alt"></i>
                                "Blog Name (required) *"
                            </label>
                            <input
                                type="text"
                                id="blog-name"
                                prop:value=blog_name
                                on:input=move |ev| set_blog_name.set(event_target_value(&ev))
                                placeholder="Enter blog name (1-64 characters)..."
                                maxlength="64"
                                prop:disabled=move || is_creating.get()
                                required
                            />
                        </div>
                        
                        // Blog Description
                        <div class="form-group">
                            <label for="blog-description">
                                <i class="fas fa-align-left"></i>
                                "Description (optional)"
                            </label>
                            <textarea
                                id="blog-description"
                                prop:value=blog_description
                                on:input=move |ev| set_blog_description.set(event_target_value(&ev))
                                placeholder="Describe your blog (max 256 characters)..."
                                maxlength="256"
                                rows="4"
                                prop:disabled=move || is_creating.get()
                            ></textarea>
                        </div>
                    </div>
                    
                    // Right side: Pixel Art Editor and Burn Amount
                    <div class="form-right">
                        <div class="pixel-art-editor">
                            <div class="pixel-art-header">
                                <label>
                                    <i class="fas fa-image"></i>
                                    "Blog Avatar"
                                </label>
                                <div class="pixel-art-controls">
                                    <select
                                        class="size-selector"
                                        prop:value=move || grid_size.get().to_string()
                                        on:change=handle_grid_size_change
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
                                        editable=true
                                        size=256
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
                        
                        // Burn Amount
                        <div class="form-group" style="margin-top: 20px;">
                            <label for="create-burn-amount">
                                <i class="fas fa-fire"></i>
                                "Burn Amount (MEMO tokens)"
                            </label>
                            <input
                                type="number"
                                id="create-burn-amount"
                                prop:value=move || burn_amount.get().to_string()
                                on:input=move |ev| {
                                    if let Ok(val) = event_target_value(&ev).parse::<u64>() {
                                        set_burn_amount.set(val);
                                    }
                                }
                                min="1"
                                prop:disabled=move || is_creating.get()
                            />
                            <small class="form-hint">
                                <i class="fas fa-wallet"></i>
                                {move || {
                                    let balance = session.with(|s| s.get_token_balance());
                                    let is_sufficient = balance >= 1.0;
                                    view! {
                                        "Minimum: 1 MEMO (Available: "
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
                            <div class="error-message" 
                                class:success=message.contains("✅")
                                class:error=message.contains("❌")
                            >
                                {message}
                            </div>
                        }.into_view()
                    } else {
                        view! { <div></div> }.into_view()
                    }
                }}
                
                // Creating status
                <Show when=move || !creating_status.get().is_empty()>
                    <div class="form-status">
                        <div class="loading-spinner"></div>
                        {creating_status}
                    </div>
                </Show>
                
                // Submit button
                <div class="form-actions">
                    <button
                        type="submit"
                        class="submit-btn"
                        prop:disabled=move || is_creating.get()
                    >
                        {move || if is_creating.get() {
                            "Creating...".to_string()
                        } else {
                            format!("🔥 Create Blog ({} MEMO)", burn_amount.get())
                        }}
                    </button>
                </div>
            </form>
        </div>
    }
}

/// Update Blog Form
#[component]
fn UpdateBlogForm(
    session: RwSignal<Session>,
    on_close: Rc<dyn Fn()>,
    on_success: Rc<dyn Fn(String)>,
) -> impl IntoView {
    let on_close_signal = create_rw_signal(Some(on_close));
    let on_success_signal = create_rw_signal(Some(on_success));
    
    // Current blog state
    let (current_blog, set_current_blog) = create_signal::<Option<BlogInfo>>(None);
    let (loading_blog, set_loading_blog) = create_signal(true);
    
    // Form state
    let (blog_name, set_blog_name) = create_signal(String::new());
    let (blog_description, set_blog_description) = create_signal(String::new());
    let (burn_amount, set_burn_amount) = create_signal(1u64);
    let (pixel_art, set_pixel_art) = create_signal(Pixel::new_with_size(16));
    let (grid_size, set_grid_size) = create_signal(16usize);
    let (is_updating, set_is_updating) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    
    // Load current blog
    create_effect(move |_| {
        spawn_local(async move {
            set_loading_blog.set(true);
            let session_read = session.get_untracked();
            if let Ok(pubkey) = session_read.get_public_key() {
                if let Ok(blog) = session_read.get_user_blog(&pubkey).await {
                    set_current_blog.set(Some(blog.clone()));
                    set_blog_name.set(blog.name.clone());
                    set_blog_description.set(blog.description.clone());
                    
                    // Try to load existing pixel art
                    if !blog.image.is_empty() {
                        if let Some(pixel) = Pixel::from_optimal_string(&blog.image) {
                            let (size, _) = pixel.dimensions();
                            set_grid_size.set(size);
                            set_pixel_art.set(pixel);
                        }
                    }
                }
            }
            set_loading_blog.set(false);
        });
    });
    
    let get_image_data = move || -> String {
        pixel_art.get().to_optimal_string()
    };
    
    let handle_close = move |_| {
        if is_updating.get() {
            return;
        }
        on_close_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };
    
    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        
        if is_updating.get() {
            return;
        }
        
        let blog = match current_blog.get() {
            Some(b) => b,
            None => {
                set_error_message.set("❌ No blog found".to_string());
                return;
            }
        };
        
        let name = blog_name.get().trim().to_string();
        if name.is_empty() || name.len() > 64 {
            set_error_message.set(format!("❌ Blog name must be 1-64 characters, got {}", name.len()));
            return;
        }
        
        let description = blog_description.get().trim().to_string();
        if description.len() > 256 {
            set_error_message.set(format!("❌ Description must be at most 256 characters, got {}", description.len()));
            return;
        }
        
        let amount = burn_amount.get();
        if amount < 1 {
            set_error_message.set("❌ Burn amount must be at least 1 MEMO".to_string());
            return;
        }
        
        let image = get_image_data();
        
        set_is_updating.set(true);
        set_error_message.set(String::new());
        
        spawn_local(async move {
            let mut session_update = session.get_untracked();
            
            match session_update.update_blog(
                Some(name),
                Some(description),
                Some(image),
                amount,
            ).await {
                Ok(signature) => {
                    session.update(|s| s.mark_balance_update_needed());
                    on_success_signal.with_untracked(|cb_opt| {
                        if let Some(callback) = cb_opt.as_ref() {
                            callback(signature);
                        }
                    });
                },
                Err(e) => {
                    set_error_message.set(format!("❌ Failed to update blog: {}", e));
                    set_is_updating.set(false);
                }
            }
        });
    };
    
    let handle_grid_size_change = move |ev: leptos::ev::Event| {
        if let Ok(size) = event_target_value(&ev).parse::<usize>() {
            if size == 16 || size == 32 {
                set_grid_size.set(size);
                set_pixel_art.set(Pixel::new_with_size(size));
            }
        }
    };
    
    // Copy string state
    let (show_copied, set_show_copied) = create_signal(false);
    
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
    
    view! {
        <div class="update-blog-form">
            <div class="form-header">
                <h3 class="form-title">
                    <i class="fas fa-edit"></i>
                    "Update Blog"
                </h3>
                <button
                    type="button"
                    class="form-close-btn"
                    on:click=handle_close
                    title="Close"
                    prop:disabled=move || is_updating.get()
                >
                    <i class="fas fa-times"></i>
                </button>
            </div>
            
            <Show
                when=move || !loading_blog.get()
                fallback=|| view! {
                    <div class="form-loading">
                        <div class="loading-spinner"></div>
                        <p>"Loading blog..."</p>
                    </div>
                }
            >
                <Show
                    when=move || current_blog.get().is_some()
                    fallback=|| view! {
                        <div class="no-blog-warning">
                            <i class="fas fa-exclamation-circle"></i>
                            <p>"No blog found to update"</p>
                        </div>
                    }
                >
                    <form class="blog-form" on:submit=handle_submit>
                        <div class="form-layout">
                            // Left side: Basic Information
                            <div class="form-left">
                                // Blog Name
                                <div class="form-group">
                                    <label for="update-blog-name">
                                        <i class="fas fa-pencil-alt"></i>
                                        "Blog Name *"
                                    </label>
                                    <input
                                        type="text"
                                        id="update-blog-name"
                                        prop:value=blog_name
                                        on:input=move |ev| set_blog_name.set(event_target_value(&ev))
                                        placeholder="Enter blog name (1-64 characters)..."
                                        maxlength="64"
                                        prop:disabled=move || is_updating.get()
                                        required
                                    />
                                </div>
                                
                                // Blog Description
                                <div class="form-group">
                                    <label for="update-blog-description">
                                        <i class="fas fa-align-left"></i>
                                        "Description"
                                    </label>
                                    <textarea
                                        id="update-blog-description"
                                        prop:value=blog_description
                                        on:input=move |ev| set_blog_description.set(event_target_value(&ev))
                                        placeholder="Describe your blog (max 256 characters)..."
                                        maxlength="256"
                                        rows="4"
                                        prop:disabled=move || is_updating.get()
                                    ></textarea>
                                </div>
                            </div>
                            
                            // Right side: Pixel Art Editor and Burn Amount
                            <div class="form-right">
                                <div class="pixel-art-editor">
                                    <div class="pixel-art-header">
                                        <label>
                                            <i class="fas fa-image"></i>
                                            "Blog Avatar"
                                        </label>
                                        <div class="pixel-art-controls">
                                            <select
                                                class="size-selector"
                                                prop:value=move || grid_size.get().to_string()
                                                on:change=handle_grid_size_change
                                                prop:disabled=move || is_updating.get()
                                            >
                                                <option value="16">"16×16 pixels"</option>
                                                <option value="32">"32×32 pixels"</option>
                                            </select>
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
                                                editable=true
                                                size=256
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
                                
                                // Burn Amount
                                <div class="form-group" style="margin-top: 20px;">
                                    <label for="update-burn-amount">
                                        <i class="fas fa-fire"></i>
                                        "Burn Amount (MEMO tokens)"
                                    </label>
                                    <input
                                        type="number"
                                        id="update-burn-amount"
                                        prop:value=move || burn_amount.get().to_string()
                                        on:input=move |ev| {
                                            if let Ok(val) = event_target_value(&ev).parse::<u64>() {
                                                set_burn_amount.set(val);
                                            }
                                        }
                                        min="1"
                                        prop:disabled=move || is_updating.get()
                                    />
                                    <small class="form-hint">
                                        <i class="fas fa-info-circle"></i>
                                        "Minimum: 1 MEMO token required to update blog"
                                    </small>
                                </div>
                            </div>
                        </div>
                        
                        // Error message
                        <Show when=move || !error_message.get().is_empty()>
                            <div class="form-error">
                                {error_message}
                            </div>
                        </Show>
                        
                        // Submit button
                        <div class="form-actions">
                            <button
                                type="submit"
                                class="submit-btn"
                                prop:disabled=move || is_updating.get()
                            >
                                {move || if is_updating.get() {
                                    "Updating...".to_string()
                                } else {
                                    format!("🔥 Update Blog ({} MEMO)", burn_amount.get())
                                }}
                            </button>
                        </div>
                    </form>
                </Show>
            </Show>
        </div>
    }
}

