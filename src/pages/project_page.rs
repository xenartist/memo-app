use leptos::*;
use crate::core::session::Session;
use crate::core::rpc_project::ProjectCreationData;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use web_sys::{HtmlInputElement, FileReader, Event, ProgressEvent, window};
use wasm_bindgen::{closure::Closure, JsCast};
use js_sys::Uint8Array;
use std::rc::Rc;
use crate::pages::pixel_view::PixelView;
use crate::core::pixel::Pixel;

/// Project row data for table display
#[derive(Clone, Debug, PartialEq)]
struct ProjectRow {
    project_id: u64,
    name: String,
    description: String,
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

/// Project Details View component - placeholder for now
#[component]
fn ProjectDetailsView(
    project: ProjectRow,
    on_back: Rc<dyn Fn()>,
    session: RwSignal<Session>,
) -> impl IntoView {
    let on_back_signal = create_rw_signal(Some(on_back));

    let handle_back = move |_| {
        on_back_signal.with_untracked(|cb_opt| {
            if let Some(callback) = cb_opt.as_ref() {
                callback();
            }
        });
    };

    view! {
        <div class="project-details-view">
            <div class="details-header">
                <button 
                    class="back-button"
                    on:click=handle_back
                    title="Back to leaderboard"
                >
                    <i class="fas fa-arrow-left"></i>
                    "Back to Projects"
                </button>
                <h1 class="project-title">{project.name.clone()}</h1>
            </div>
            
            <div class="details-content">
                <div class="placeholder-content">
                    <div class="placeholder-text">
                        <h2>
                            <i class="fas fa-tools"></i>
                            "Project Details"
                        </h2>
                        <p>"This is a placeholder for the project details page."</p>
                        <p>"Project ID: " {project.project_id.to_string()}</p>
                        <p>"More details will be implemented here later."</p>
                    </div>
                </div>
            </div>
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
