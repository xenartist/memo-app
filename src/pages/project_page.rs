use leptos::*;
use crate::core::session::Session;
use crate::core::rpc_project::{ProjectBurnLeaderboardResponse, ProjectInfo};
use wasm_bindgen_futures::spawn_local;

/// Project row data for table display
#[derive(Clone, Debug)]
struct ProjectRow {
    project_id: u64,
    name: String,
    website: String,
    burned_amount: u64,
    last_memo_time: i64,
    rank: u8,
}

/// Project page component - displays projects in a simple table format
#[component]
pub fn ProjectPage(
    session: RwSignal<Session>,
) -> impl IntoView {
    let (projects, set_projects) = create_signal::<Vec<ProjectRow>>(vec![]);
    let (loading, set_loading) = create_signal(true);
    let (error_message, set_error_message) = create_signal::<Option<String>>(None);

    // Load projects from burn leaderboard
    create_effect(move |_| {
        let session_clone = session;
        spawn_local(async move {
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
                                    website: project_info.website,
                                    burned_amount: entry.burned_amount,
                                    last_memo_time: project_info.last_memo_time,
                                    rank: entry.rank,
                                });
                            },
                            Err(e) => {
                                log::warn!("Failed to fetch project {} info: {}", entry.project_id, e);
                            }
                        }
                    }
                    
                    // Sort by rank (already should be sorted, but just to be sure)
                    project_rows.sort_by(|a, b| a.rank.cmp(&b.rank));
                    
                    set_projects.set(project_rows);
                },
                Err(e) => {
                    log::error!("Failed to fetch project burn leaderboard: {}", e);
                    set_error_message.set(Some(format!("Failed to load projects: {}", e)));
                }
            }
            
            set_loading.set(false);
        });
    });

    view! {
        <div class="project-page">
            <div class="project-header">
                <h1>"X1.WIKI"</h1>
                <p class="project-subtitle">"Top 100 Projects on X1 Blockchain"</p>
            </div>
            
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
                                                <th>"Website"</th>
                                                <th>"Burned (MEMO)"</th>
                                                <th>"Last Activity"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {project_list.into_iter().map(|project| {
                                                let burned_tokens = project.burned_amount / 1_000_000;
                                                let last_activity = format_timestamp(project.last_memo_time);
                                                let website_display = if project.website.is_empty() {
                                                    "-".to_string()
                                                } else {
                                                    project.website.clone()
                                                };
                                                
                                                view! {
                                                    <tr class="project-row">
                                                        <td class="rank-cell">
                                                            <span class="rank-badge">#{project.rank.to_string()}</span>
                                                        </td>
                                                        <td class="id-cell">{project.project_id.to_string()}</td>
                                                        <td class="name-cell">
                                                            <span class="project-name">{project.name}</span>
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
                                                            <span class="burned-amount">{format_number_with_commas(burned_tokens)}</span>
                                                        </td>
                                                        <td class="time-cell">
                                                            <span class="last-activity">{last_activity}</span>
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
    }
}

/// Format timestamp to human readable format
fn format_timestamp(timestamp: i64) -> String {
    if timestamp == 0 {
        return "-".to_string();
    }
    
    // Convert to JavaScript Date and format
    let js_date = js_sys::Date::new(&wasm_bindgen::JsValue::from(timestamp as f64 * 1000.0));
    let options = js_sys::Object::new();
    js_sys::Reflect::set(&options, &"year".into(), &"numeric".into()).unwrap();
    js_sys::Reflect::set(&options, &"month".into(), &"short".into()).unwrap();
    js_sys::Reflect::set(&options, &"day".into(), &"numeric".into()).unwrap();
    js_sys::Reflect::set(&options, &"hour".into(), &"2-digit".into()).unwrap();
    js_sys::Reflect::set(&options, &"minute".into(), &"2-digit".into()).unwrap();
    
    let formatted = js_date.to_locale_string("en-US", &options);
    formatted.as_string().unwrap_or_else(|| "Invalid Date".to_string())
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
