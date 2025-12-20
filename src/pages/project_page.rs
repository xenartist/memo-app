use leptos::*;
use crate::core::session::Session;
use crate::core::rpc_project::{ProjectCreationData, ProjectBurnMessage};
use crate::core::rpc_base::RpcConnection;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use web_sys::{HtmlInputElement, FileReader, Event, ProgressEvent, window};
use wasm_bindgen::{closure::Closure, JsCast};
use js_sys::Uint8Array;
use wasm_bindgen::JsValue;
use std::rc::Rc;
use crate::pages::pixel_view::{PixelView, LazyPixelView};
use crate::core::pixel::Pixel;

/// Devlog message status for UI display
#[derive(Debug, Clone, Copy, PartialEq)]
enum DevlogStatus {
    Sending,
    Sent,
    Failed,
}

/// Parsed devlog data from JSON message
#[derive(Debug, Clone, PartialEq)]
struct ParsedDevlog {
    title: String,
    content: String,
    image: String,
}

impl ParsedDevlog {
    /// Parse devlog from JSON message string
    fn from_message(message: &str) -> Option<Self> {
        // Try to parse as JSON devlog format: {"type":"devlog","title":"...","content":"...","image":"..."}
        if !message.contains("\"type\":\"devlog\"") {
            return None;
        }
        
        // Simple JSON parsing (avoiding external dependency)
        let title = Self::extract_json_field(message, "title").unwrap_or_default();
        let content = Self::extract_json_field(message, "content").unwrap_or_default();
        let image = Self::extract_json_field(message, "image").unwrap_or_default();
        
        Some(Self { title, content, image })
    }
    
    /// Extract a field value from JSON string
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
        // Unescape the string
        Some(value.replace("\\\"", "\"").replace("\\\\", "\\"))
    }
}

/// Local devlog message for immediate UI display
#[derive(Debug, Clone, PartialEq)]
struct LocalDevlogMessage {
    message: ProjectBurnMessage,
    parsed: Option<ParsedDevlog>,
    status: DevlogStatus,
    is_local: bool, // true if this is a local message not yet confirmed on chain
}

impl LocalDevlogMessage {
    /// Create a new local devlog for immediate UI display
    fn new_local(burner: String, title: String, content: String, image: String, burn_amount: u64) -> Self {
        let message_json = format!(
            r#"{{"type":"devlog","title":"{}","content":"{}","image":"{}"}}"#,
            title.replace('\\', "\\\\").replace('"', "\\\""),
            content.replace('\\', "\\\\").replace('"', "\\\""),
            image.replace('\\', "\\\\").replace('"', "\\\"")
        );
        
        Self {
            message: ProjectBurnMessage {
                signature: format!("local_devlog_{}", js_sys::Date::now() as u64),
                burner,
                message: message_json.clone(),
                timestamp: (js_sys::Date::now() / 1000.0) as i64,
                slot: 0,
                burn_amount: burn_amount * 1_000_000, // Convert to lamports
            },
            parsed: Some(ParsedDevlog { title, content, image }),
            status: DevlogStatus::Sending,
            is_local: true,
        }
    }
    
    /// Create from chain message
    fn from_chain_message(message: ProjectBurnMessage) -> Self {
        let parsed = ParsedDevlog::from_message(&message.message);
        Self {
            message,
            parsed,
            status: DevlogStatus::Sent,
            is_local: false,
        }
    }
}

/// Project row data for table display
#[derive(Clone, Debug, PartialEq)]
struct ProjectRow {
    project_id: u64,
    name: String,
    description: String,
    image: String,
    website: String,
    burned_amount: u64,
    last_memo_time: i64,
    rank: u8,
    creator: String, // Base58 encoded pubkey
}

/// Page view state
#[derive(Clone, Debug, PartialEq)]
enum PageView {
    Leaderboard,
    ProjectDetails(ProjectRow),
}

/// Project page component - displays projects in a simple table format
#[component]
pub fn ProjectPage(
    session: RwSignal<Session>,
) -> impl IntoView {
    let (projects, set_projects) = create_signal::<Vec<ProjectRow>>(vec![]);
    let (loading, set_loading) = create_signal(true);
    let (error_message, set_error_message) = create_signal::<Option<String>>(None);
    
    // Page navigation state
    let (current_view, set_current_view) = create_signal(PageView::Leaderboard);
    
    // Create Project Dialog states
    let (show_create_dialog, set_show_create_dialog) = create_signal(false);
    
    // Countdown state
    let (countdown_seconds, set_countdown_seconds) = create_signal::<Option<i32>>(None);

    // Function to load/refresh projects data  
    let load_projects_data = create_action(move |_: &()| {
        let session_clone = session;
        async move {
            set_loading.set(true);
            set_error_message.set(None);
            
            let session_read = session_clone.get_untracked();
            
            match session_read.get_project_burn_leaderboard().await {
                Ok(leaderboard) => {
                    log::info!("Fetched burn leaderboard with {} projects", leaderboard.entries.len());
                    
                    let mut project_rows = Vec::new();
                    
                    // Fetch detailed info for each project in leaderboard
                    for entry in leaderboard.entries {
                        match session_read.get_project_info(entry.project_id).await {
                            Ok(project_info) => {
                                project_rows.push(ProjectRow {
                                    project_id: entry.project_id,
                                    name: project_info.name,
                                    description: project_info.description,
                                    image: project_info.image,
                                    website: project_info.website,
                                    burned_amount: entry.burned_amount,
                                    last_memo_time: project_info.last_memo_time,
                                    rank: entry.rank,
                                    creator: project_info.creator,
                                });
                            },
                            Err(e) => {
                                log::warn!("Failed to fetch project {} info: {}", entry.project_id, e);
                            }
                        }
                    }
                    
                    // Sort by burned_amount in descending order (highest burn first)
                    // and reassign ranks based on actual burn amounts
                    project_rows.sort_by(|a, b| b.burned_amount.cmp(&a.burned_amount));
                    
                    // Reassign ranks based on sorted order
                    for (index, project) in project_rows.iter_mut().enumerate() {
                        project.rank = (index + 1) as u8;
                    }
                    
                    set_projects.set(project_rows);
                },
                Err(e) => {
                    log::error!("Failed to fetch project burn leaderboard: {}", e);
                    set_error_message.set(Some(format!("Failed to load projects: {}", e)));
                }
            }
            
            set_loading.set(false);
        }
    });

    // Load projects on component mount
    create_effect(move |_| {
        load_projects_data.dispatch(());
    });

    // Function to open create project dialog
    let open_create_dialog = move |_| {
        set_show_create_dialog.set(true);
    };

    // Function to close create project dialog
    let close_create_dialog = move || {
        set_show_create_dialog.set(false);
    };

    // Function to view project details
    let view_project_details = move |project: ProjectRow| {
        set_current_view.set(PageView::ProjectDetails(project));
    };

    // Function to go back to leaderboard
    let back_to_leaderboard = move || {
        set_current_view.set(PageView::Leaderboard);
    };

    // Function to handle successful project creation
    let on_project_created = move |signature: String, project_id: u64| {
        log::info!("Project created successfully! ID: {}, Signature: {}", project_id, signature);
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
            TimeoutFuture::new(20_000).await; // Wait 20 seconds
            
            log::info!("Refreshing project list after project creation...");
            load_projects_data.dispatch(());
        });
    };

    // Function to handle project creation error
    let on_project_creation_error = move |error: String| {
        log::error!("Project creation failed: {}", error);
        // Error is already handled by the form itself
    };

    view! {
        <div class="project-page">
            {move || {
                match current_view.get() {
                    PageView::Leaderboard => {
                        view! {
                            <div class="leaderboard-view">
                                <div class="project-header">
                                    <div class="header-content">
                                        <div class="header-text">
                                            <h1>
                                                "X1.Wiki"
                                            </h1>
                                            <p class="project-subtitle">"Top 100 Projects on X1 Blockchain"</p>
                                        </div>
                                        <div class="header-actions">
                                            <button 
                                                class="new-project-button"
                                                on:click=open_create_dialog
                                                disabled=move || loading.get()
                                                title="Create new project"
                                            >
                                                <i class="fas fa-plus"></i>
                                                "New Project"
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
                                                "Project created successfully! Leaderboard will refresh in "
                                                <strong>{move || countdown_seconds.get().unwrap_or(0).to_string()}</strong>
                                                " seconds..."
                                            </span>
                                        </div>
                                    </div>
                                </Show>
                                
                                <div class="project-content">
                                    {move || {
                                        if loading.get() {
                                            view! {
                                                <div class="loading-state">
                                                    <p>"Loading projects..."</p>
                                                </div>
                                            }.into_view()
                                        } else if let Some(error) = error_message.get() {
                                            view! {
                                                <div class="error-state">
                                                    <p>"Error: "{error}</p>
                                                </div>
                                            }.into_view()
                                        } else {
                                            let project_list = projects.get();
                                            if project_list.is_empty() {
                                                view! {
                                                    <div class="empty-state">
                                                        <p>"No projects found in burn leaderboard."</p>
                                                    </div>
                                                }.into_view()
                                            } else {
                                                view! {
                                                    <div class="project-table-container">
                                                        <table class="project-table">
                                                            <thead>
                                                                <tr>
                                                                    <th>"Rank"</th>
                                                                    <th>"ID"</th>
                                                                    <th>"Name"</th>
                                                                    <th>"Description"</th>
                                                                    <th>"Website"</th>
                                                                    <th>"Burned (MEMO)"</th>
                                                                    <th>"Details"</th>
                                                                </tr>
                                                            </thead>
                                                            <tbody>
                                                                {project_list.into_iter().map(|project| {
                                                                    let burned_tokens = project.burned_amount / 1_000_000;
                                                                    let website_display = if project.website.is_empty() {
                                                                        "-".to_string()
                                                                    } else {
                                                                        project.website.clone()
                                                                    };
                                                                    let description_display = truncate_description(&project.description);
                                                                    let project_clone = project.clone();
                                                                    
                                                                    view! {
                                                                        <tr class="project-row">
                                                                            <td class="rank-cell">
                                                                                {
                                                                                    let rank_num = project.rank;
                                                                                    if rank_num == 1 {
                                                                                        view! {
                                                                                            <span class="rank-icon rank-1st">
                                                                                                <i class="fas fa-trophy"></i>
                                                                                                <span class="rank-number">"1"</span>
                                                                                            </span>
                                                                                        }.into_view()
                                                                                    } else if rank_num == 2 {
                                                                                        view! {
                                                                                            <span class="rank-icon rank-2nd">
                                                                                                <i class="fas fa-medal"></i>
                                                                                                <span class="rank-number">"2"</span>
                                                                                            </span>
                                                                                        }.into_view()
                                                                                    } else if rank_num == 3 {
                                                                                        view! {
                                                                                            <span class="rank-icon rank-3rd">
                                                                                                <i class="fas fa-medal"></i>
                                                                                                <span class="rank-number">"3"</span>
                                                                                            </span>
                                                                                        }.into_view()
                                                                                    } else if rank_num >= 4 && rank_num <= 10 {
                                                                                        view! {
                                                                                            <span class="rank-icon rank-top10">
                                                                                                <i class="fas fa-fire"></i>
                                                                                                <span class="rank-number">{rank_num.to_string()}</span>
                                                                                            </span>
                                                                                        }.into_view()
                                                                                    } else {
                                                                                        view! {
                                                                                            <span class="rank-icon rank-others">
                                                                                                <i class="fas fa-fire"></i>
                                                                                                <span class="rank-number">{rank_num.to_string()}</span>
                                                                                            </span>
                                                                                        }.into_view()
                                                                                    }
                                                                                }
                                                                            </td>
                                                                            <td class="id-cell">{project.project_id.to_string()}</td>
                                                                            <td class="name-cell">
                                                                                <span class="project-name">{project.name}</span>
                                                                            </td>
                                                                            <td class="description-cell">
                                                                                <span class="project-description">{description_display}</span>
                                                                            </td>
                                                                            <td class="website-cell">
                                                                                {if !project.website.is_empty() {
                                                                                    view! {
                                                                                        <a 
                                                                                            href={project.website} 
                                                                                            target="_blank" 
                                                                                            rel="noopener noreferrer"
                                                                                            class="website-link"
                                                                                        >
                                                                                            {website_display}
                                                                                        </a>
                                                                                    }.into_view()
                                                                                } else {
                                                                                    view! {
                                                                                        <span class="no-website">"-"</span>
                                                                                    }.into_view()
                                                                                }}
                                                                            </td>
                                                                            <td class="burned-cell">
                                                                                <i class="fas fa-fire burned-fire-icon"></i>
                                                                                <span class="burned-number">{format_number_with_commas(burned_tokens)}</span>
                                                                            </td>
                                                                            <td class="actions-cell">
                                                                                <button 
                                                                                    class="details-button"
                                                                                    on:click=move |_| view_project_details(project_clone.clone())
                                                                                    title="View project details"
                                                                                >
                                                                                    <i class="fas fa-info-circle"></i>
                                                                                    "Details"
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
                    PageView::ProjectDetails(project) => {
                        view! {
                            <ProjectDetailsView
                                project=project
                                on_back=Rc::new(back_to_leaderboard)
                                session=session
                            />
                        }.into_view()
                    }
                }
            }}

            // Create Project Dialog
            <Show when=move || show_create_dialog.get()>
                <div class="modal-overlay">
                    <CreateProjectForm
                        session=session
                        on_close=Rc::new(close_create_dialog)
                        on_success=Rc::new(on_project_created)
                        on_error=Rc::new(on_project_creation_error)
                    />
                </div>
            </Show>
        </div>
    }
}

/// Shorten address for display (e.g., "ABC123...XYZ9")
fn shorten_address(addr: &str) -> String {
    if addr.len() > 12 {
        format!("{}...{}", &addr[..6], &addr[addr.len()-4..])
    } else {
        addr.to_string()
    }
}

/// Project Details View component - displays project information in a clean card layout
#[component]
fn ProjectDetailsView(
    project: ProjectRow,
    on_back: Rc<dyn Fn()>,
    session: RwSignal<Session>,
) -> impl IntoView {
    let on_back_signal = create_rw_signal(Some(on_back));
    
    // Store project data as reactive signal for updates
    let project_data = create_rw_signal(project.clone());
    
    // Create a reactive accessor for current project
    let current_project = move || project_data.get();

    let handle_back = move |_| {
        on_back_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };

    // Reactive computed values based on project_data
    let burned_display = move || {
        let proj = current_project();
        let burned_tokens = proj.burned_amount / 1_000_000;
        format_number_with_commas(burned_tokens)
    };
    
    let last_memo_display = move || {
        let proj = current_project();
        if proj.last_memo_time > 0 {
            let date = web_sys::js_sys::Date::new(&JsValue::from_f64(proj.last_memo_time as f64 * 1000.0));
            format!(
                "{}-{:02}-{:02} {:02}:{:02}",
                date.get_full_year(),
                date.get_month() + 1,
                date.get_date(),
                date.get_hours(),
                date.get_minutes()
            )
        } else {
            "Never".to_string()
        }
    };

    // Check if current user is the creator - make these reactive
    let is_creator = move || {
        let proj = current_project();
        session.with(|s| {
            match s.get_public_key() {
                Ok(pubkey) => pubkey == proj.creator,
                Err(_) => false,
            }
        })
    };
    
    let is_creator_for_devlog_btn = move || {
        let proj = current_project();
        session.with(|s| {
            match s.get_public_key() {
                Ok(pubkey) => pubkey == proj.creator,
                Err(_) => false,
            }
        })
    };
    
    let is_creator_for_hint = move || {
        let proj = current_project();
        session.with(|s| {
            match s.get_public_key() {
                Ok(pubkey) => pubkey == proj.creator,
                Err(_) => false,
            }
        })
    };
    
    // Create a reactive memo for is_creator check (used in devlog list)
    let is_creator_memo = create_memo(move |_| {
        let proj = current_project();
        session.with(|s| {
            match s.get_public_key() {
                Ok(pubkey) => pubkey == proj.creator,
                Err(_) => false,
            }
        })
    });
    
    // Update dialog state
    let (show_update_dialog, set_show_update_dialog) = create_signal(false);
    
    // Refresh countdown state (for showing countdown after update)
    let (refresh_countdown, set_refresh_countdown) = create_signal(0u32);
    let (is_refreshing, set_is_refreshing) = create_signal(false);
    
    // Refresh trigger - increment this to force reload all data
    let (refresh_trigger, set_refresh_trigger) = create_signal(0u32);
    
    // Devlog dialog state
    let (show_devlog_dialog, set_show_devlog_dialog) = create_signal(false);
    
    // Devlog list state
    let (devlogs, set_devlogs) = create_signal::<Vec<LocalDevlogMessage>>(vec![]);
    let (devlogs_loading, set_devlogs_loading) = create_signal(true);
    let (devlogs_error, set_devlogs_error) = create_signal::<Option<String>>(None);
    
    // Store project_id for devlog operations
    let project_id_for_devlogs = project.project_id;
    
    // Load devlogs on mount and when refresh_trigger changes
    {
        let project_id = project.project_id;
        create_effect(move |_| {
            // Watch refresh_trigger to reload when it changes
            let _ = refresh_trigger.get();
            
            spawn_local(async move {
                set_devlogs_loading.set(true);
                set_devlogs_error.set(None);
                
                let rpc = RpcConnection::new();
                match rpc.get_project_burn_messages(project_id, 50, None).await {
                    Ok(response) => {
                        // Filter only devlog messages and convert to LocalDevlogMessage
                        let devlog_messages: Vec<LocalDevlogMessage> = response.messages
                            .into_iter()
                            .filter(|msg| msg.message.contains("\"type\":\"devlog\""))
                            .map(LocalDevlogMessage::from_chain_message)
                            .collect();
                        
                        log::info!("Loaded {} devlogs for project {}", devlog_messages.len(), project_id);
                        set_devlogs.set(devlog_messages);
                    },
                    Err(e) => {
                        log::error!("Failed to load devlogs: {}", e);
                        set_devlogs_error.set(Some(format!("Failed to load devlogs: {}", e)));
                    }
                }
                set_devlogs_loading.set(false);
            });
        });
    }
    
    // Creator display name - start with shortened address, then try to fetch username
    let creator_addr_for_display = project.creator.clone();
    let (creator_display, set_creator_display) = create_signal(shorten_address(&creator_addr_for_display));
    let (creator_username, set_creator_username) = create_signal::<Option<String>>(None);
    
    // Fetch creator's profile to get username
    {
        let creator_addr = creator_addr_for_display.clone();
        create_effect(move |_| {
            let addr = creator_addr.clone();
            spawn_local(async move {
                let rpc = crate::core::rpc_base::RpcConnection::new();
                match rpc.get_profile(&addr).await {
                    Ok(Some(profile)) => {
                        log::info!("Found creator profile: {}", profile.username);
                        set_creator_display.set(profile.username.clone());
                        set_creator_username.set(Some(profile.username));
                    },
                    Ok(None) => {
                        log::info!("No profile found for creator: {}", addr);
                    },
                    Err(e) => {
                        log::warn!("Failed to fetch creator profile: {}", e);
                    }
                }
            });
        });
    }

    // Copy address to clipboard
    let copy_address = {
        let address = project.creator.clone();
        move |_| {
            if let Some(window) = window() {
                let clipboard = window.navigator().clipboard();
                let _ = clipboard.write_text(&address);
            }
        }
    };
    
    // Open update dialog
    let open_update_dialog = move |_| {
        set_show_update_dialog.set(true);
    };
    
    // Close update dialog
    let close_update_dialog = move || {
        set_show_update_dialog.set(false);
    };
    
    // Handle update success - just close dialog, no need to wait here
    let on_update_success = move |_signature: String| {
        log::info!("Project updated successfully, starting refresh countdown");
        set_show_update_dialog.set(false);
        
        // Start countdown and refresh
        set_is_refreshing.set(true);
        set_refresh_countdown.set(20);
        
        let project_id = project.project_id;
        let original_rank = project.rank;
        
        // Countdown timer
        spawn_local(async move {
            for remaining in (1..=20).rev() {
                set_refresh_countdown.set(remaining);
                TimeoutFuture::new(1_000).await;
            }
            set_refresh_countdown.set(0);
        });
        
        // Wait 20 seconds then refresh project details
        spawn_local(async move {
            log::info!("Waiting 20 seconds for blockchain to update...");
            TimeoutFuture::new(20_000).await;
            
            log::info!("Fetching updated project info...");
            let rpc = RpcConnection::new();
            match rpc.get_project_info(project_id).await {
                Ok(project_info) => {
                    log::info!("Successfully fetched updated project data, reloading details page");
                    // Create updated ProjectRow
                    let updated_project = ProjectRow {
                        project_id: project_info.project_id,
                        name: project_info.name,
                        description: project_info.description,
                        image: project_info.image,
                        website: project_info.website,
                        burned_amount: project_info.burned_amount,
                        last_memo_time: project_info.last_memo_time,
                        rank: original_rank,
                        creator: project_info.creator,
                    };
                    
                    // Update project data - this will trigger all UI updates
                    project_data.set(updated_project);
                    
                    // Trigger refresh for devlogs and other data
                    set_refresh_trigger.update(|n| *n += 1);
                },
                Err(e) => {
                    log::error!("Failed to refresh project data: {}", e);
                }
            }
            
            set_is_refreshing.set(false);
        });
    };

    // Open devlog dialog
    let open_devlog_dialog = move |_| {
        set_show_devlog_dialog.set(true);
    };
    
    // Close devlog dialog
    let close_devlog_dialog = move || {
        set_show_devlog_dialog.set(false);
    };
    
    // Handle devlog success
    let on_devlog_success = move |_signature: String| {
        log::info!("Devlog posted successfully!");
        set_show_devlog_dialog.set(false);
    };

    view! {
        <div class="project-details-page">
            <div class="project-details-container">
                // Back button
                <button 
                    class="pd-back-btn"
                    on:click=handle_back
                    title="Back to leaderboard"
                >
                    <i class="fas fa-arrow-left"></i>
                    "Back to Projects"
                </button>
                
                // Refresh countdown banner (shown after update)
                <Show when=move || is_refreshing.get()>
                    <div style="
                        background: #d1ecf1;
                        color: #0c5460;
                        padding: 20px;
                        border-radius: 12px;
                        border: 1px solid #bee5eb;
                        margin: 20px 0;
                        text-align: center;
                        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
                    ">
                        <div style="font-size: 18px; font-weight: 600; margin-bottom: 12px;">
                            <i class="fas fa-sync-alt fa-spin" style="margin-right: 8px;"></i>
                            "Project updated successfully!"
                        </div>
                        <div style="font-size: 48px; font-weight: 700; margin: 16px 0; color: #0c5460;">
                            {move || refresh_countdown.get()}
                        </div>
                        <div style="font-size: 14px; opacity: 0.8;">
                            "Waiting for blockchain synchronization..."
                        </div>
                    </div>
                </Show>
                
                // Project Detail Card
                <div class="project-detail-card">
                    // Card Header with Image, Name, Rank and Update Button
                    <div class="pd-card-header">
                        <div class="pd-header-content">
                            // Project Image
                            {move || {
                                let proj = current_project();
                                if !proj.image.is_empty() {
                                    if proj.image.starts_with("c:") || proj.image.starts_with("n:") {
                                        view! {
                                            <div class="pd-project-avatar">
                                                <LazyPixelView
                                                    art={proj.image.clone()}
                                                    size=80
                                                />
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <div class="pd-project-avatar">
                                                <img src={proj.image.clone()} alt="Project Image" />
                                            </div>
                                        }.into_view()
                                    }
                                } else {
                                    view! {
                                        <div class="pd-project-avatar placeholder">
                                            <i class="fas fa-cube"></i>
                                        </div>
                                    }.into_view()
                                }
                            }}
                            
                            // Name and Rank
                            <div class="project-name-section">
                                <h1 class="project-detail-name">{move || current_project().name}</h1>
                                {move || {
                                    let proj = current_project();
                                    view! {
                                        <span class={format!("rank-badge rank-{}", if proj.rank <= 3 { proj.rank.to_string() } else if proj.rank <= 10 { "top10".to_string() } else { "other".to_string() })}>
                                            {if proj.rank == 1 {
                                                view! { <><i class="fas fa-trophy"></i> " #1"</> }.into_view()
                                            } else if proj.rank <= 3 {
                                                view! { <><i class="fas fa-medal"></i> {format!(" #{}", proj.rank)}</> }.into_view()
                                            } else {
                                                view! { <><i class="fas fa-fire"></i> {format!(" #{}", proj.rank)}</> }.into_view()
                                            }}
                                        </span>
                                    }
                                }}
                            </div>
                            
                            // Update button (only visible to creator)
                            <Show when=move || is_creator()>
                                <button 
                                    class="pd-update-btn"
                                    on:click=open_update_dialog
                                    title="Update project"
                                >
                                    <i class="fas fa-edit"></i>
                                    "Update"
                                </button>
                            </Show>
                        </div>
                    </div>
                    
                    // Card Body with horizontal layout
                    <div class="pd-card-body">
                        // Left side - Main info
                        <div class="info-left">
                            // Project ID
                            <div class="detail-field">
                                <div class="pd-field-icon">
                                    <i class="fas fa-hashtag"></i>
                                </div>
                                <div class="pd-field-content">
                                    <span class="pd-field-label">"Project ID"</span>
                                    <span class="pd-field-value mono">{move || current_project().project_id.to_string()}</span>
                                </div>
                            </div>
                            
                            // Description
                            <div class="detail-field description-field">
                                <div class="pd-field-icon">
                                    <i class="fas fa-align-left"></i>
                                </div>
                                <div class="pd-field-content">
                                    <span class="pd-field-label">"Description"</span>
                                    <span class="pd-field-value">
                                        {move || {
                                            let proj = current_project();
                                            if proj.description.is_empty() {
                                                "-".to_string()
                                            } else {
                                                proj.description.clone()
                                            }
                                        }}
                                    </span>
                                </div>
                            </div>
                            
                            // Website
                            <div class="detail-field">
                                <div class="pd-field-icon">
                                    <i class="fas fa-globe"></i>
                                </div>
                                <div class="pd-field-content">
                                    <span class="pd-field-label">"Website"</span>
                                    {move || {
                                        let proj = current_project();
                                        if !proj.website.is_empty() {
                                            view! {
                                                <a 
                                                    href={proj.website.clone()} 
                                                    target="_blank" 
                                                    rel="noopener noreferrer"
                                                    class="pd-field-value link"
                                                >
                                                    {proj.website.clone()}
                                                    <i class="fas fa-external-link-alt"></i>
                                                </a>
                                            }.into_view()
                                        } else {
                                            view! {
                                                <span class="pd-field-value muted">"-"</span>
                                            }.into_view()
                                        }
                                    }}
                                </div>
                            </div>
                        </div>
                        
                        // Right side - Stats
                        <div class="info-right">
                            // Burned Amount
                            <div class="pd-stat-card">
                                <div class="pd-stat-icon">
                                    <i class="fas fa-fire"></i>
                                </div>
                                <div class="pd-stat-content">
                                    <span class="pd-stat-label">"Burned"</span>
                                    <span class="pd-stat-value">{burned_display}" MEMO"</span>
                                </div>
                            </div>
                            
                            // Last Memo Time
                            <div class="pd-stat-card">
                                <div class="pd-stat-icon">
                                    <i class="fas fa-clock"></i>
                                </div>
                                <div class="pd-stat-content">
                                    <span class="pd-stat-label">"Last Memo"</span>
                                    <span class="pd-stat-value">{last_memo_display}</span>
                                </div>
                            </div>
                        </div>
                    </div>
                    
                    // Card Footer - Creator
                    <div class="pd-card-footer">
                        <div class="creator-section">
                            <span class="creator-label">
                                <i class="fas fa-user"></i>
                                "Created by"
                            </span>
                            <div class="creator-info">
                                <span class="pd-creator-name">{move || creator_display.get()}</span>
                                // Show address hint if we have a username
                                {move || {
                                    let proj = current_project();
                                    if creator_username.get().is_some() {
                                        view! {
                                            <span class="pd-address-hint">
                                                "(" {shorten_address(&proj.creator)} ")"
                                            </span>
                                        }.into_view()
                                    } else {
                                        view! { <span></span> }.into_view()
                                    }
                                }}
                                <button 
                                    class="pd-copy-btn"
                                    on:click=copy_address
                                    title="Copy full address to clipboard"
                                >
                                    <i class="fas fa-copy"></i>
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
                
                // Devlog Section (outside project card)
                <div class="devlog-section">
                    // Section Header with New Devlog button
                    <div class="devlog-section-header">
                        <h2 class="devlog-section-title">
                            <i class="fas fa-book-open"></i>
                            "Development Logs"
                        </h2>
                        // New Devlog button (only visible to creator)
                        <Show when=move || is_creator_for_devlog_btn()>
                            <button 
                                class="pd-devlog-btn"
                                on:click=open_devlog_dialog
                                title="Post a new devlog"
                            >
                                <i class="fas fa-plus"></i>
                                "New Devlog"
                            </button>
                        </Show>
                    </div>
                    
                    // Devlog list
                    <div class="devlog-list">
                        {move || {
                            if devlogs_loading.get() {
                                // Loading state
                                view! {
                                    <div class="devlog-loading">
                                        <i class="fas fa-spinner fa-spin"></i>
                                        <p>"Loading development logs..."</p>
                                    </div>
                                }.into_view()
                            } else if let Some(error) = devlogs_error.get() {
                                // Error state
                                view! {
                                    <div class="devlog-error">
                                        <i class="fas fa-exclamation-triangle"></i>
                                        <p>{error}</p>
                                    </div>
                                }.into_view()
                            } else if devlogs.get().is_empty() {
                                // Empty state
                                view! {
                                    <div class="devlog-empty-state">
                                        <i class="fas fa-scroll"></i>
                                        <p>"No development logs yet"</p>
                                        <Show when=move || is_creator_memo.get()>
                                            <span class="devlog-empty-hint">"Click 'New Devlog' to share your first update!"</span>
                                        </Show>
                                    </div>
                                }.into_view()
                            } else {
                                // Devlog cards
                                let logs = devlogs.get();
                                view! {
                                    <For
                                        each=move || devlogs.get()
                                        key=|devlog| devlog.message.signature.clone()
                                        children=move |devlog| {
                                            view! {
                                                <DevlogCard 
                                                    devlog=devlog.clone()
                                                    session=session
                                                    devlogs=set_devlogs
                                                    project_id=project_id_for_devlogs
                                                />
                                            }
                                        }
                                    />
                                }.into_view()
                            }
                        }}
                    </div>
                </div>
            </div>
            
            // Update Project Dialog
            <Show when=move || show_update_dialog.get()>
                <div class="modal-overlay">
                    <UpdateProjectForm
                        session=session
                        project=project_data
                        on_close=Rc::new(close_update_dialog)
                        on_success=Rc::new(on_update_success)
                    />
                </div>
            </Show>
            
            // Devlog Dialog
            <Show when=move || show_devlog_dialog.get()>
                <div class="modal-overlay">
                    <DevlogForm
                        session=session
                        project=project_data
                        devlogs=set_devlogs
                        on_close=Rc::new(close_devlog_dialog)
                        on_success=Rc::new(on_devlog_success)
                    />
                </div>
            </Show>
        </div>
    }
}

/// Devlog Card component - displays a single devlog entry
#[component]
fn DevlogCard(
    devlog: LocalDevlogMessage,
    session: RwSignal<Session>,
    devlogs: WriteSignal<Vec<LocalDevlogMessage>>,
    project_id: u64,
) -> impl IntoView {
    let status = devlog.status;
    let is_local = devlog.is_local;
    let signature = devlog.message.signature.clone();
    let burner = devlog.message.burner.clone();
    let timestamp = devlog.message.timestamp;
    let burn_amount = devlog.message.burn_amount;
    let message_raw = devlog.message.message.clone();
    
    // Get parsed devlog data
    let parsed = devlog.parsed.clone();
    let title = parsed.as_ref().map(|p| p.title.clone()).unwrap_or_else(|| "Untitled".to_string());
    let content = parsed.as_ref().map(|p| p.content.clone()).unwrap_or_default();
    let image = parsed.as_ref().map(|p| p.image.clone()).unwrap_or_default();
    
    // Clone for retry
    let title_for_retry = title.clone();
    let content_for_retry = content.clone();
    let image_for_retry = image.clone();
    let signature_for_retry = signature.clone();
    
    // Format timestamp
    let time_display = if timestamp > 0 {
        let date = web_sys::js_sys::Date::new(&JsValue::from_f64(timestamp as f64 * 1000.0));
        format!(
            "{}-{:02}-{:02} {:02}:{:02}",
            date.get_full_year(),
            date.get_month() + 1,
            date.get_date(),
            date.get_hours(),
            date.get_minutes()
        )
    } else {
        "Just now".to_string()
    };
    
    // Format burn amount
    let burn_display = format!("{}", burn_amount / 1_000_000);
    
    // Handle retry
    let handle_retry = move |_| {
        let title = title_for_retry.clone();
        let content = content_for_retry.clone();
        let image = image_for_retry.clone();
        let sig = signature_for_retry.clone();
        let proj_id = project_id;
        
        // Update status to Sending
        devlogs.update(|logs| {
            if let Some(devlog) = logs.iter_mut().find(|d| d.message.signature == sig) {
                devlog.status = DevlogStatus::Sending;
            }
        });
        
        spawn_local(async move {
            let devlog_data = DevlogData::new(title.clone(), content.clone(), image.clone());
            let message = devlog_data.to_json();
            
            let mut session_update = session.get_untracked();
            let result = session_update.burn_tokens_for_project(
                proj_id,
                420, // Minimum burn amount for retry
                &message,
            ).await;
            
            match result {
                Ok(new_signature) => {
                    devlogs.update(|logs| {
                        if let Some(devlog) = logs.iter_mut().find(|d| d.message.signature == sig) {
                            devlog.status = DevlogStatus::Sent;
                            devlog.message.signature = new_signature;
                        }
                    });
                    
                    session.update(|s| {
                        s.mark_balance_update_needed();
                    });
                },
                Err(_) => {
                    devlogs.update(|logs| {
                        if let Some(devlog) = logs.iter_mut().find(|d| d.message.signature == sig) {
                            devlog.status = DevlogStatus::Failed;
                        }
                    });
                }
            }
        });
    };
    
    view! {
        <div 
            class="devlog-card"
            class:devlog-sending=move || status == DevlogStatus::Sending
            class:devlog-failed=move || status == DevlogStatus::Failed
        >
            // Card Header
            <div class="devlog-card-header">
                <h3 class="devlog-title">{title}</h3>
                <div class="devlog-meta">
                    <span class="devlog-time">
                        <i class="fas fa-clock"></i>
                        {time_display}
                    </span>
                    <span class="devlog-burn">
                        <i class="fas fa-fire"></i>
                        {burn_display}" MEMO"
                    </span>
                </div>
            </div>
            
            // Card Body - Horizontal layout
            <div class="devlog-card-body">
                // Image section (left side)
                {if !image.is_empty() && (image.starts_with("c:") || image.starts_with("n:")) {
                    view! {
                        <div class="devlog-image-section">
                            <div class="devlog-image">
                                <LazyPixelView
                                    art={image.clone()}
                                    size=100
                                />
                            </div>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="devlog-image-section">
                            <div class="devlog-image-placeholder">
                                <i class="fas fa-image"></i>
                                <span>"No image"</span>
                            </div>
                        </div>
                    }.into_view()
                }}

                // Content section (right side)
                <div class="devlog-content-section">
                    {if !content.is_empty() {
                        view! {
                            <p class="devlog-content">{content}</p>
                        }.into_view()
                    } else {
                        view! {
                            <p class="devlog-content devlog-content-empty">"No content"</p>
                        }.into_view()
                    }}
                </div>
            </div>
            
            // Status indicator (for local messages) - rendered based on initial status
            {if status == DevlogStatus::Sending {
                view! {
                    <div class="devlog-status sending">
                        <i class="fas fa-spinner fa-spin"></i>
                        " Sending..."
                    </div>
                }.into_view()
            } else if status == DevlogStatus::Failed {
                view! {
                    <div class="devlog-status failed">
                        <i class="fas fa-exclamation-circle"></i>
                        " Failed to send"
                        <button 
                            class="retry-btn"
                            on:click=handle_retry
                        >
                            <i class="fas fa-redo"></i>
                            " Retry"
                        </button>
                    </div>
                }.into_view()
            } else {
                view! { <div></div> }.into_view()
            }}
        </div>
    }
}

/// Devlog data structure for calculating memo size
#[derive(Clone, Debug)]
struct DevlogData {
    title: String,
    content: String,
    image: String,
}

impl DevlogData {
    fn new(title: String, content: String, image: String) -> Self {
        Self { title, content, image }
    }
    
    /// Convert to JSON string for storage in message field
    fn to_json(&self) -> String {
        format!(
            r#"{{"type":"devlog","title":"{}","content":"{}","image":"{}"}}"#,
            self.title.replace('\\', "\\\\").replace('"', "\\\""),
            self.content.replace('\\', "\\\\").replace('"', "\\\""),
            self.image.replace('\\', "\\\\").replace('"', "\\\"")
        )
    }
    
    /// Calculate final memo size (Borsh + Base64) for devlog
    fn calculate_final_memo_size(&self, project_id: u64, burner: &str, burn_amount: u64) -> Result<usize, String> {
        use crate::core::rpc_project::{ProjectBurnData, BurnMemo};
        use crate::core::constants::BURN_MEMO_VERSION;
        use borsh::BorshSerialize;
        
        let message = self.to_json();
        
        // Create ProjectBurnData
        let burn_data = ProjectBurnData::new(
            project_id,
            burner.to_string(),
            message,
        );
        
        // Serialize ProjectBurnData to Borsh
        let payload_bytes = burn_data.try_to_vec()
            .map_err(|e| format!("Failed to serialize ProjectBurnData: {}", e))?;
        
        // Create BurnMemo with the payload
        let burn_memo = BurnMemo {
            version: BURN_MEMO_VERSION,
            burn_amount,
            payload: payload_bytes,
        };
        
        // Serialize BurnMemo to Borsh
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| format!("Failed to serialize BurnMemo: {}", e))?;
        
        // Encode to Base64 (this is what actually gets sent)
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        Ok(memo_data_base64.len())
    }
}

/// Devlog Form component - allows creator to post development logs
#[component]
fn DevlogForm(
    session: RwSignal<Session>,
    project: RwSignal<ProjectRow>,
    devlogs: WriteSignal<Vec<LocalDevlogMessage>>,
    on_close: Rc<dyn Fn()>,
    on_success: Rc<dyn Fn(String)>,
) -> impl IntoView {
    let on_close_signal = create_rw_signal(Some(on_close));
    let on_success_signal = create_rw_signal(Some(on_success));
    
    // Get project data
    let original_project = project.get_untracked();
    let project_id = original_project.project_id;
    
    // Form state signals
    let (devlog_title, set_devlog_title) = create_signal(String::new());
    let (devlog_content, set_devlog_content) = create_signal(String::new());
    let (burn_amount, set_burn_amount) = create_signal(420u64); // Minimum 420 tokens for burn_for_project
    let (pixel_art, set_pixel_art) = create_signal(Pixel::new_with_size(16));
    let (grid_size, set_grid_size) = create_signal(16usize);
    
    // UI state signals
    let (is_posting, set_is_posting) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    let (show_copied, set_show_copied) = create_signal(false);
    
    // Get current image data
    let get_image_data = move || -> String {
        pixel_art.get().to_optimal_string()
    };
    
    // Get burner pubkey
    let get_burner_pubkey = move || -> String {
        session.with(|s| s.get_public_key().unwrap_or_default())
    };

    // Calculate memo size in real time (69-800 bytes)
    let calculate_memo_size = move || -> (usize, bool, String) {
        let title = devlog_title.get().trim().to_string();
        let content = devlog_content.get().trim().to_string();
        let image_data = get_image_data();
        let amount = burn_amount.get() * 1_000_000; // lamports
        let burner = get_burner_pubkey();

        let devlog_data = DevlogData::new(title, content, image_data);

        match devlog_data.calculate_final_memo_size(project_id, &burner, amount) {
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

    // Handle form submission
    let handle_submit = move |ev: leptos::leptos_dom::ev::SubmitEvent| {
        ev.prevent_default();

        if is_posting.get() {
            return;
        }

        let title = devlog_title.get().trim().to_string();
        let content = devlog_content.get().trim().to_string();
        let image = get_image_data();
        let amount = burn_amount.get();

        // Validation
        if title.is_empty() || title.len() > 64 {
            set_error_message.set(format!("❌ Devlog title must be 1-64 characters, got {}", title.len()));
            return;
        }
        if content.len() > 500 {
            set_error_message.set(format!("❌ Devlog content must be at most 500 characters, got {}", content.len()));
            return;
        }
        if amount < 420 {
            set_error_message.set("❌ Burn amount must be at least 420 MEMO tokens".to_string());
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

        set_is_posting.set(true);
        set_error_message.set(String::new());

        // Get user pubkey for local message
        let user_pubkey = session.with_untracked(|s| s.get_public_key().unwrap_or_default());
        
        // 1. Create local devlog for immediate UI display (optimistic update)
        let local_devlog = LocalDevlogMessage::new_local(
            user_pubkey.clone(),
            title.clone(),
            content.clone(),
            image.clone(),
            amount,
        );
        let local_signature = local_devlog.message.signature.clone();
        
        // Add to devlogs list immediately (at the beginning)
        devlogs.update(|logs| {
            logs.insert(0, local_devlog);
        });
        
        // Clear form and close dialog
        set_devlog_title.set(String::new());
        set_devlog_content.set(String::new());
        set_pixel_art.set(Pixel::new_with_size(16));
        
        // Create devlog message (JSON format) for sending
        let devlog_data = DevlogData::new(title.clone(), content.clone(), image.clone());
        let message = devlog_data.to_json();
        let proj_id = project_id;

        // 2. Send to blockchain
        spawn_local(async move {
            TimeoutFuture::new(100).await;
            
            let mut session_update = session.get_untracked();
            let result = session_update.burn_tokens_for_project(
                proj_id,
                amount,
                &message,
            ).await;

            set_is_posting.set(false);

            match result {
                Ok(signature) => {
                    // 3. Update local devlog status to Sent
                    devlogs.update(|logs| {
                        if let Some(devlog) = logs.iter_mut().find(|d| {
                            d.is_local && 
                            d.message.signature == local_signature
                        }) {
                            devlog.status = DevlogStatus::Sent;
                            devlog.message.signature = signature.clone();
                        }
                    });
                    
                    session.update(|s| {
                        s.mark_balance_update_needed();
                    });

                    on_success_signal.with_untracked(|cb_opt| {
                        if let Some(callback) = cb_opt.as_ref() {
                            callback(signature);
                        }
                    });
                },
                Err(e) => {
                    // 4. Update local devlog status to Failed
                    devlogs.update(|logs| {
                        if let Some(devlog) = logs.iter_mut().find(|d| {
                            d.is_local && 
                            d.message.signature == local_signature
                        }) {
                            devlog.status = DevlogStatus::Failed;
                        }
                    });
                    
                    set_error_message.set(format!("❌ Failed to post devlog: {}", e));
                }
            }
        });
    };

    // Handle close
    let handle_close = move |_| {
        on_close_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
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

    // Copy pixel art string
    let copy_string = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        let art_string = pixel_art.get().to_optimal_string();
        if let Some(window) = window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(&art_string);
            set_show_copied.set(true);
            spawn_local(async move {
                TimeoutFuture::new(2000).await;
                set_show_copied.set(false);
            });
        }
    };

    view! {
        <div class="devlog-form">
            <div class="form-header">
                <h3 class="form-title">
                    <i class="fas fa-book-open"></i>
                    "New Devlog"
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
                        // Devlog Title
                        <div class="form-group">
                            <label for="devlog-title">
                                <i class="fas fa-heading"></i>
                                "Devlog Title"
                                <span class="required">*</span>
                            </label>
                            <input
                                type="text"
                                id="devlog-title"
                                prop:value=devlog_title
                                on:input=move |ev| set_devlog_title.set(event_target_value(&ev))
                                placeholder="Enter devlog title (1-64 characters)..."
                                maxlength="64"
                                prop:disabled=move || is_posting.get()
                            />
                            <small class="char-count">
                                {move || format!("{}/64 characters", devlog_title.get().len())}
                            </small>
                        </div>

                        // Devlog Content
                        <div class="form-group">
                            <label for="devlog-content">
                                <i class="fas fa-align-left"></i>
                                "Content"
                            </label>
                            <textarea
                                id="devlog-content"
                                prop:value=devlog_content
                                on:input=move |ev| set_devlog_content.set(event_target_value(&ev))
                                placeholder="Write your development log here (max 500 characters)..."
                                maxlength="500"
                                rows="6"
                                prop:disabled=move || is_posting.get()
                            ></textarea>
                            <small class="char-count">
                                {move || format!("{}/500 characters", devlog_content.get().len())}
                            </small>
                        </div>
                    </div>

                    // Right side: Image and Burn Amount
                    <div class="form-right">
                        <div class="pixel-art-editor">
                            <div class="pixel-art-header">
                                <label>
                                    <i class="fas fa-image"></i>
                                    "Devlog Image (Optional)"
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
                                        "Import"
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
                            <label for="devlog-burn-amount">
                                <i class="fas fa-fire"></i>
                                "Burn Amount (MEMO)"
                            </label>
                            <input
                                type="number"
                                id="devlog-burn-amount"
                                prop:value=burn_amount
                                on:input=move |ev| {
                                    let input = event_target::<HtmlInputElement>(&ev);
                                    if let Ok(value) = input.value().parse::<u64>() {
                                        set_burn_amount.set(value.max(420));
                                    }
                                }
                                min="420"
                                prop:disabled=move || is_posting.get()
                            />
                            <small class="form-hint">
                                <i class="fas fa-wallet"></i>
                                {move || {
                                    let balance = session.with(|s| s.get_token_balance());
                                    view! {
                                        "Minimum: 420 MEMO (Available: "
                                        <span class={if balance >= 420.0 { "balance-sufficient" } else { "balance-insufficient" }}>
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
                <div class="memo-size-indicator devlog">
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
                        class="post-devlog-btn"
                        prop:disabled=move || {
                            is_posting.get() ||
                            devlog_title.get().trim().is_empty() ||
                            devlog_title.get().len() > 64 ||
                            devlog_content.get().len() > 500 ||
                            burn_amount.get() < 420 ||
                            session.with(|s| s.get_token_balance()) < burn_amount.get() as f64 ||
                            !calculate_memo_size().1 // Check if memo size is valid
                        }
                    >
                        <i class="fas fa-paper-plane"></i>
                        {move || {
                            if is_posting.get() {
                                "Posting Devlog...".to_string()
                            } else {
                                format!("Post Devlog (Burn {} MEMO)", burn_amount.get())
                            }
                        }}
                    </button>
                </div>
            </form>
        </div>
    }
}

/// Update Project Form component - allows creator to update project details
#[component]
fn UpdateProjectForm(
    session: RwSignal<Session>,
    project: RwSignal<ProjectRow>,
    on_close: Rc<dyn Fn()>,
    on_success: Rc<dyn Fn(String)>,
) -> impl IntoView {
    let on_close_signal = create_rw_signal(Some(on_close));
    let on_success_signal = create_rw_signal(Some(on_success));
    
    // Get original project data
    let original_project = project.get_untracked();
    
    // Original values for comparison
    let original_name = original_project.name.clone();
    let original_description = original_project.description.clone();
    let original_image = original_project.image.clone();
    let original_website = original_project.website.clone();
    
    // Parse original image to pixel art
    let original_pixel_art = if original_image.starts_with("c:") || original_image.starts_with("n:") {
        Pixel::from_optimal_string(&original_image).unwrap_or_else(|| Pixel::new_with_size(16))
    } else {
        Pixel::new_with_size(16)
    };
    let (original_grid_size, _) = original_pixel_art.dimensions();
    
    // Form state signals - initialized with original values
    let (project_name, set_project_name) = create_signal(original_name.clone());
    let (project_description, set_project_description) = create_signal(original_description.clone());
    let (project_website, set_project_website) = create_signal(original_website.clone());
    let (burn_amount, set_burn_amount) = create_signal(42069u64); // Minimum 42,069 tokens for update (same as contract requirement)
    let (pixel_art, set_pixel_art) = create_signal(original_pixel_art.clone());
    let (grid_size, set_grid_size) = create_signal(original_grid_size);
    
    // UI state signals
    let (is_updating, set_is_updating) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    let (show_copied, set_show_copied) = create_signal(false);
    
    // Original values for change detection (stored as signals for reactive comparison)
    let original_name_signal = create_rw_signal(original_name.clone());
    let original_description_signal = create_rw_signal(original_description.clone());
    let original_website_signal = create_rw_signal(original_website.clone());
    let original_pixel_art_signal = create_rw_signal(original_pixel_art.clone());
    
    // Change detection
    let name_changed = move || project_name.get() != original_name_signal.get();
    let description_changed = move || project_description.get() != original_description_signal.get();
    let website_changed = move || project_website.get() != original_website_signal.get();
    let image_changed = move || pixel_art.get().to_optimal_string() != original_pixel_art_signal.get().to_optimal_string();
    
    let has_changes = move || {
        name_changed() || description_changed() || website_changed() || image_changed()
    };
    
    // Get current image data
    let get_image_data = move || -> String {
        pixel_art.get().to_optimal_string()
    };

    // Calculate memo size in real time (same rule as create: 69-800 bytes)
    let calculate_memo_size = move || -> (usize, bool, String) {
        let name = project_name.get().trim().to_string();
        let description = project_description.get().trim().to_string();
        let image_data = get_image_data();
        let website = project_website.get().trim().to_string();
        let tags: Vec<String> = vec![]; // tags not editable in update for now
        let amount = burn_amount.get() * 1_000_000; // lamports

        let project_data = ProjectCreationData::new(
            original_project.project_id,
            name,
            description,
            image_data,
            website,
            tags,
        );

        match project_data.calculate_final_memo_size(amount) {
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

    // Handle form submission
    let handle_submit = move |ev: leptos::leptos_dom::ev::SubmitEvent| {
        ev.prevent_default();

        if is_updating.get() || !has_changes() {
            return;
        }

        let name = project_name.get().trim().to_string();
        let description = project_description.get().trim().to_string();
        let image = get_image_data();
        let website = project_website.get().trim().to_string();
        let amount = burn_amount.get();
        let proj_id = original_project.project_id;

        // Validation
        if name.is_empty() || name.len() > 64 {
            set_error_message.set(format!("❌ Project name must be 1-64 characters, got {}", name.len()));
            return;
        }
        if description.len() > 256 {
            set_error_message.set(format!("❌ Description must be at most 256 characters, got {}", description.len()));
            return;
        }
        if website.len() > 128 {
            set_error_message.set(format!("❌ Website must be at most 128 characters, got {}", website.len()));
            return;
        }
        if amount < 42069 {
            set_error_message.set("❌ Burn amount must be at least 42,069 MEMO tokens".to_string());
            return;
        }

        // Check balance
        let token_balance = session.with_untracked(|s| s.get_token_balance());
        if token_balance < amount as f64 {
            set_error_message.set(format!("❌ Insufficient balance. Required: {} MEMO, Available: {:.2} MEMO", amount, token_balance));
            return;
        }

        set_is_updating.set(true);
        set_error_message.set(String::new());

        // Prepare optional fields - only send changed ones
        let name_opt = if name_changed() { Some(name) } else { None };
        let desc_opt = if description_changed() { Some(description) } else { None };
        let image_opt = if image_changed() { Some(image) } else { None };
        let website_opt = if website_changed() { Some(website) } else { None };

        spawn_local(async move {
            TimeoutFuture::new(100).await;
            
            let mut session_update = session.get_untracked();
            let result = session_update.update_project(
                proj_id,
                name_opt,
                desc_opt,
                image_opt,
                website_opt,
                None, // tags not editable for now
                amount,
            ).await;

            set_is_updating.set(false);

            match result {
                Ok(signature) => {
                    session.update(|s| {
                        s.mark_balance_update_needed();
                    });

                    // Immediately trigger success callback and close dialog
                    set_is_updating.set(false);
                    on_success_signal.with_untracked(|cb_opt| {
                        if let Some(callback) = cb_opt.as_ref() {
                            callback(signature);
                        }
                    });
                },
                Err(e) => {
                    set_error_message.set(format!("❌ Failed to update project: {}", e));
                }
            }
        });
    };

    // Handle close
    let handle_close = move |_| {
        on_close_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
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

    // Copy pixel art string
    let copy_string = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        let art_string = pixel_art.get().to_optimal_string();
        if let Some(window) = window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(&art_string);
            set_show_copied.set(true);
            spawn_local(async move {
                TimeoutFuture::new(2000).await;
                set_show_copied.set(false);
            });
        }
    };

    view! {
        <div class="update-project-form">
            <div class="form-header">
                <h3 class="form-title">
                    <i class="fas fa-edit"></i>
                    "Update Project"
                </h3>
                <button
                    type="button"
                    class="form-close-btn"
                    on:click=handle_close
                    prop:disabled=move || is_updating.get()
                    title="Close"
                >
                    <i class="fas fa-times"></i>
                </button>
            </div>
            
            <form class="project-form" on:submit=handle_submit>
                <div class="form-layout">
                    // Left side: Basic Information
                    <div class="form-left">
                        // Project Name
                        <div class="form-group">
                            <label for="update-project-name">
                                <i class="fas fa-pencil-alt"></i>
                                "Project Name"
                                {move || if name_changed() {
                                    view! { 
                                        <span class="changed-indicator">
                                            <i class="fas fa-edit"></i>
                                            "Modified"
                                        </span> 
                                    }.into_view()
                                } else {
                                    view! { <span></span> }.into_view()
                                }}
                            </label>
                            <input
                                type="text"
                                id="update-project-name"
                                prop:value=project_name
                                on:input=move |ev| set_project_name.set(event_target_value(&ev))
                                placeholder="Enter project name (1-64 characters)..."
                                maxlength="64"
                                prop:disabled=move || is_updating.get()
                                class:changed=name_changed
                            />
                        </div>

                        // Project Description
                        <div class="form-group">
                            <label for="update-project-description">
                                <i class="fas fa-align-left"></i>
                                "Description"
                                {move || if description_changed() {
                                    view! { 
                                        <span class="changed-indicator">
                                            <i class="fas fa-edit"></i>
                                            "Modified"
                                        </span> 
                                    }.into_view()
                                } else {
                                    view! { <span></span> }.into_view()
                                }}
                            </label>
                            <textarea
                                id="update-project-description"
                                prop:value=project_description
                                on:input=move |ev| set_project_description.set(event_target_value(&ev))
                                placeholder="Enter project description (max 256 characters)..."
                                maxlength="256"
                                rows="3"
                                prop:disabled=move || is_updating.get()
                                class:changed=description_changed
                            ></textarea>
                        </div>

                        // Project Website
                        <div class="form-group">
                            <label for="update-project-website">
                                <i class="fas fa-link"></i>
                                "Website"
                                {move || if website_changed() {
                                    view! { 
                                        <span class="changed-indicator">
                                            <i class="fas fa-edit"></i>
                                            "Modified"
                                        </span> 
                                    }.into_view()
                                } else {
                                    view! { <span></span> }.into_view()
                                }}
                            </label>
                            <input
                                type="text"
                                id="update-project-website"
                                prop:value=project_website
                                on:input=move |ev| set_project_website.set(event_target_value(&ev))
                                placeholder="Enter website URL (max 128 characters)..."
                                maxlength="128"
                                prop:disabled=move || is_updating.get()
                                class:changed=website_changed
                            />
                        </div>
                    </div>

                    // Right side: Project Image
                    <div class="form-right">
                        <div class="pixel-art-editor">
                            <div class="pixel-art-header">
                                <label>
                                    <i class="fas fa-image"></i>
                                    "Project Image"
                                    {move || if image_changed() {
                                        view! { 
                                            <span class="changed-indicator">
                                                <i class="fas fa-edit"></i>
                                                "Modified"
                                            </span> 
                                        }.into_view()
                                    } else {
                                        view! { <span></span> }.into_view()
                                    }}
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
                                        prop:disabled=move || is_updating.get()
                                    >
                                        <option value="16">"16×16 pixels"</option>
                                        <option value="32">"32×32 pixels"</option>
                                    </select>
                                    <button 
                                        type="button"
                                        class="import-btn"
                                        on:click=handle_import
                                        prop:disabled=move || is_updating.get()
                                    >
                                        <i class="fas fa-upload"></i>
                                        "Import"
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
                                        size=200
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
                            <label for="update-burn-amount">
                                <i class="fas fa-fire"></i>
                                "Burn Amount (MEMO)"
                            </label>
                            <input
                                type="number"
                                id="update-burn-amount"
                                prop:value=burn_amount
                                on:input=move |ev| {
                                    let input = event_target::<HtmlInputElement>(&ev);
                                    if let Ok(value) = input.value().parse::<u64>() {
                                        set_burn_amount.set(value.max(42069));
                                    }
                                }
                                min="42069"
                                prop:disabled=move || is_updating.get()
                            />
                            <small class="form-hint">
                                <i class="fas fa-wallet"></i>
                                {move || {
                                    let balance = session.with(|s| s.get_token_balance());
                                    view! {
                                        "Minimum: 42,069 MEMO (Available: "
                                        <span class={if balance >= 42069.0 { "balance-sufficient" } else { "balance-insufficient" }}>
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
                <div class="memo-size-indicator update">
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

                // Pending Changes Summary
                {move || if has_changes() {
                    view! {
                        <div class="changes-summary">
                            <h4>
                                <i class="fas fa-exclamation-circle"></i>
                                "Pending Changes"
                            </h4>
                            <ul>
                                {move || if name_changed() {
                                    view! {
                                        <li>
                                            "Name: "
                                            <span class="old-value">{original_name_signal.get()}</span>
                                            " → "
                                            <span class="new-value">{project_name.get()}</span>
                                        </li>
                                    }.into_view()
                                } else {
                                    view! { <li style="display:none"></li> }.into_view()
                                }}
                                {move || if description_changed() {
                                    let old_desc = original_description_signal.get();
                                    let new_desc = project_description.get();
                                    view! {
                                        <li>
                                            "Description: "
                                            <span class="old-value">{if old_desc.len() > 30 { format!("{}...", &old_desc[..30]) } else { old_desc }}</span>
                                            " → "
                                            <span class="new-value">{if new_desc.len() > 30 { format!("{}...", &new_desc[..30]) } else { new_desc }}</span>
                                        </li>
                                    }.into_view()
                                } else {
                                    view! { <li style="display:none"></li> }.into_view()
                                }}
                                {move || if website_changed() {
                                    view! {
                                        <li>
                                            "Website: "
                                            <span class="old-value">{original_website_signal.get()}</span>
                                            " → "
                                            <span class="new-value">{project_website.get()}</span>
                                        </li>
                                    }.into_view()
                                } else {
                                    view! { <li style="display:none"></li> }.into_view()
                                }}
                                {move || if image_changed() {
                                    view! {
                                        <li>
                                            "Image: "
                                            <span class="old-value">"(previous)"</span>
                                            " → "
                                            <span class="new-value">"(new image)"</span>
                                        </li>
                                    }.into_view()
                                } else {
                                    view! { <li style="display:none"></li> }.into_view()
                                }}
                            </ul>
                        </div>
                    }.into_view()
                } else {
                    view! { <div></div> }.into_view()
                }}

                // Error message only
                {move || {
                    let error_msg = error_message.get();
                    if !error_msg.is_empty() {
                        view! {
                            <div class="error-message">{error_msg}</div>
                        }.into_view()
                    } else {
                        view! { <div></div> }.into_view()
                    }
                }}

                // Submit button
                <div class="button-group">
                    <button
                        type="submit"
                        class="update-project-btn"
                        prop:disabled=move || {
                            is_updating.get() ||
                            !has_changes() ||
                            project_name.get().trim().is_empty() ||
                            burn_amount.get() < 42069 ||
                            session.with(|s| s.get_token_balance()) < burn_amount.get() as f64
                        }
                    >
                        <i class="fas fa-save"></i>
                        {move || {
                            if is_updating.get() {
                                "Updating...".to_string()
                            } else {
                                format!("Update Project (Burn {} MEMO)", burn_amount.get())
                            }
                        }}
                    </button>
                </div>
            </form>
        </div>
    }
}

/// Create Project Form component - strictly reference chat page design
#[component]
fn CreateProjectForm(
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
    let (project_name, set_project_name) = create_signal(String::new());
    let (project_description, set_project_description) = create_signal(String::new());
    let (project_website, set_project_website) = create_signal(String::new());
    let (project_tags, set_project_tags) = create_signal(String::new()); // comma-separated tags
    let (burn_amount, set_burn_amount) = create_signal(42069u64); // default 42,069 tokens (minimum required)
    let (pixel_art, set_pixel_art) = create_signal(Pixel::new_with_size(16)); // default 16x16
    
    // UI state signals
    let (is_creating, set_is_creating) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    let (show_copied, set_show_copied) = create_signal(false);
    let (creating_status, set_creating_status) = create_signal(String::new());

    // Grid size for pixel art
    let (grid_size, set_grid_size) = create_signal(16usize);

    // Parse tags from comma-separated string
    let parse_tags = move || -> Vec<String> {
        project_tags.get()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .take(4) // Maximum 4 tags
            .collect()
    };

    // Create combined image data
    let get_image_data = move || -> String {
        pixel_art.get().to_optimal_string()
    };

    // Calculate current memo size in bytes (Borsh + Base64) - 参考rpc_project.rs中的实现
    let calculate_memo_size = move || -> (usize, bool, String) {
        let name = project_name.get().trim().to_string();
        let description = project_description.get().trim().to_string();
        let image_data = get_image_data();
        let website = project_website.get().trim().to_string();
        let tags = parse_tags();
        let amount = burn_amount.get() * 1_000_000; // Convert to lamports
        
        // Create temporary ProjectCreationData for size calculation
        let project_data = ProjectCreationData::new(
            0, // temporary project_id
            name,
            description,
            image_data,
            website,
            tags,
        );
        
        match project_data.calculate_final_memo_size(amount) {
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

    // Handle form submission - 参考chat page的实现，包含100ms sleep
    let handle_submit = move |ev: leptos::leptos_dom::ev::SubmitEvent| {
        ev.prevent_default();

        if is_creating.get() {
            return;
        }

        // Validate form
        let name = project_name.get().trim().to_string();
        let description = project_description.get().trim().to_string();
        let image = get_image_data();
        let website = project_website.get().trim().to_string();
        let tags = parse_tags();
        let amount = burn_amount.get();

        // Validation
        if name.is_empty() || name.len() > 64 {
            set_error_message.set(format!("❌ Project name must be 1-64 characters, got {}", name.len()));
            return;
        }
        if description.len() > 256 {
            set_error_message.set(format!("❌ Project description must be at most 256 characters, got {}", description.len()));
            return;
        }
        if image.len() > 256 {
            set_error_message.set(format!("❌ Project image must be at most 256 characters, got {}", image.len()));
            return;
        }
        if website.len() > 128 {
            set_error_message.set(format!("❌ Project website must be at most 128 characters, got {}", website.len()));
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

        // Check balance
        let token_balance = session.with_untracked(|s| s.get_token_balance());
        if token_balance < amount as f64 {
            set_error_message.set(format!("❌ Insufficient balance. Required: {} MEMO, Available: {:.2} MEMO", amount, token_balance));
            return;
        }

        // Set UI state
        set_is_creating.set(true);
        set_creating_status.set("Creating project...".to_string());
        set_error_message.set(String::new());

        // Create project
        spawn_local(async move {
            // Give UI time to update the loading state - 重要的100ms sleep防止UI卡顿
            TimeoutFuture::new(100).await;
            
            let mut session_update = session.get_untracked();
            let result = session_update.create_project(
                &name,
                &description,
                &image,
                &website,
                tags,
                amount, // session层会转换为lamports
            ).await;

            set_is_creating.set(false);
            set_creating_status.set(String::new());

            match result {
                Ok((signature, project_id)) => {
                    // Update session to trigger balance refresh
                    session.update(|s| {
                        s.mark_balance_update_needed();
                    });

                    on_success_signal.with_untracked(|cb_opt| {
                        if let Some(callback) = cb_opt.as_ref() {
                            callback(signature, project_id);
                        }
                    });
                },
                Err(e) => {
                    let error_msg = format!("Failed to create project: {}", e);
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

    // Handle close
    let handle_close = move |_| {
        on_close_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };

    // Handle image import - 参考chat page的实现
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
        <div class="create-project-form">
            // Header with title and close button - 完全参考chat page设计
            <div class="form-header">
                <h3 class="form-title">
                    <i class="fas fa-rocket"></i>
                    "Create New Project"
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
                        // Project Name
                        <div class="form-group">
                            <label for="project-name">
                                <i class="fas fa-pencil-alt"></i>
                                "Project Name (required) *"
                            </label>
                            <input
                                type="text"
                                id="project-name"
                                prop:value=project_name
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_project_name.set(value);
                                }
                                placeholder="Enter project name (1-64 characters)..."
                                maxlength="64"
                                prop:disabled=move || is_creating.get()
                                required
                            />
                        </div>

                        // Project Description
                        <div class="form-group">
                            <label for="project-description">
                                <i class="fas fa-align-left"></i>
                                "Project Description (optional)"
                            </label>
                            <textarea
                                id="project-description"
                                prop:value=project_description
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_project_description.set(value);
                                }
                                placeholder="Enter project description (max 256 characters)..."
                                maxlength="256"
                                rows="3"
                                prop:disabled=move || is_creating.get()
                            ></textarea>
                        </div>

                        // Project Website
                        <div class="form-group">
                            <label for="project-website">
                                <i class="fas fa-link"></i>
                                "Project Website (optional)"
                            </label>
                            <input
                                type="text"
                                id="project-website"
                                prop:value=project_website
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_project_website.set(value);
                                }
                                placeholder="Enter website URL (max 128 characters)..."
                                maxlength="128"
                                prop:disabled=move || is_creating.get()
                            />
                        </div>

                        // Tags
                        <div class="form-group">
                            <label for="project-tags">
                                <i class="fas fa-tags"></i>
                                "Tags (optional)"
                            </label>
                            <input
                                type="text"
                                id="project-tags"
                                prop:value=project_tags
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_project_tags.set(value);
                                }
                                placeholder="Enter tags separated by commas (max 4 tags, 32 chars each)..."
                                prop:disabled=move || is_creating.get()
                            />
                            <small class="form-hint">
                                <i class="fas fa-info-circle"></i>
                                "Example: blockchain, defi, gaming, social"
                            </small>
                        </div>
                    </div>

                    // Right side: Project Image (Pixel Art) and Burn Amount - 完全参考chat page设计
                    <div class="form-right">
                        <div class="pixel-art-editor">
                            <div class="pixel-art-header">
                                <label>
                                    <i class="fas fa-image"></i>
                                    "Project Image"
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

                // Memo size indicator - 完全参考chat page设计
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

                // Submit button - 完全参考chat page设计
                <div class="button-group">
                    <button
                        type="submit"
                        class="create-project-btn"
                        prop:disabled=move || {
                            is_creating.get() ||
                            project_name.get().trim().is_empty() ||
                            project_name.get().len() > 64 ||
                            project_description.get().len() > 256 ||
                            project_website.get().len() > 128 ||
                            parse_tags().len() > 4 ||
                            burn_amount.get() < 42069 ||
                            session.with(|s| s.get_token_balance()) < burn_amount.get() as f64 ||
                            !calculate_memo_size().1 // 检查memo size是否有效
                        }
                    >
                        <i class="fas fa-rocket"></i>
                        {move || {
                            if is_creating.get() {
                                "Creating Project...".to_string()
                            } else {
                                format!("Create Project (Burn {} MEMO)", burn_amount.get())
                            }
                        }}
                    </button>
                </div>
            </form>
        </div>
    }
}

/// Format number with comma separators
fn format_number_with_commas(num: u64) -> String {
    let num_str = num.to_string();
    let mut result = String::new();
    let chars: Vec<char> = num_str.chars().collect();
    
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*ch);
    }
    
    result
}

/// Truncate description to first 128 bytes and add ellipsis if longer
fn truncate_description(description: &str) -> String {
    if description.is_empty() {
        return "-".to_string();
    }
    
    let bytes = description.as_bytes();
    if bytes.len() <= 128 {
        description.to_string()
    } else {
        // Find the last complete UTF-8 character boundary within 128 bytes
        let mut end = 128;
        while end > 0 && !description.is_char_boundary(end) {
            end -= 1;
        }
        
        if end == 0 {
            "...".to_string()
        } else {
            format!("{}...", &description[..end])
        }
    }
}
