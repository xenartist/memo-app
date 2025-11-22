use leptos::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

// simple log storage
static mut LOG_ENTRIES: Vec<LogEntry> = Vec::new();

pub fn add_log_entry(level: &str, message: &str) {
    let timestamp = {
        let date = web_sys::js_sys::Date::new_0();
        let hours = date.get_hours();
        let minutes = date.get_minutes();
        let seconds = date.get_seconds();
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    };
    
    let entry = LogEntry {
        timestamp,
        level: level.to_string(),
        message: message.to_string(),
    };
    
    unsafe {
        LOG_ENTRIES.push(entry);
        // keep latest 100 logs
        if LOG_ENTRIES.len() > 100 {
            LOG_ENTRIES.remove(0);
        }
    }
}

pub fn get_log_entries() -> Vec<LogEntry> {
    unsafe { LOG_ENTRIES.clone() }
}

pub fn clear_logs() {
    unsafe { LOG_ENTRIES.clear(); }
}

#[component]
pub fn LogView() -> impl IntoView {
    let (is_collapsed, set_is_collapsed) = create_signal(true);
    let (refresh_trigger, set_refresh_trigger) = create_signal(0);
    
    // periodically refresh log display
    create_effect(move |_| {
        use gloo_timers::callback::Timeout;
        
        let trigger_clone = set_refresh_trigger.clone();
        Timeout::new(1000, move || {
            trigger_clone.update(|n| *n += 1);
        }).forget();
    });

    let toggle_collapse = move |_| {
        set_is_collapsed.update(|collapsed| *collapsed = !*collapsed);
    };

    let clear_logs_click = move |e: ev::MouseEvent| {
        e.stop_propagation();
        clear_logs();
        set_refresh_trigger.update(|n| *n += 1);
    };

    view! {
        <div class="log-view" style="
            position: fixed;
            bottom: 0;
            left: 0;
            right: 0;
            background: white;
            border-top: 1px solid #ddd;
            z-index: 1000;
            max-height: 300px;
            display: flex;
            flex-direction: column;
            box-shadow: 0 -2px 10px rgba(0,0,0,0.1);
        ">
            <div 
                class="log-header" 
                on:click=toggle_collapse 
                style="
                    padding: 8px 16px;
                    background: #f8f9fa;
                    border-bottom: 1px solid #ddd;
                    cursor: pointer;
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    user-select: none;
                "
            >
                <div class="log-title" style="display: flex; align-items: center; gap: 8px;">
                    <i class="fas fa-terminal" style="color: #666;"></i>
                    <span style="font-weight: 600; color: #333;">Debug Logs</span>
                    <span class="log-count" style="
                        background: #6c757d;
                        color: white;
                        padding: 2px 6px;
                        border-radius: 10px;
                        font-size: 12px;
                        font-weight: 500;
                    ">
                        {move || {
                            refresh_trigger.get();
                            format!("{}", get_log_entries().len())
                        }}
                    </span>
                </div>
                <div class="log-controls" style="display: flex; align-items: center; gap: 8px;">
                    <button 
                        class="log-clear-btn"
                        on:click=clear_logs_click
                        title="Clear logs"
                        style="
                            padding: 4px 8px;
                            background: #dc3545;
                            color: white;
                            border: none;
                            border-radius: 4px;
                            cursor: pointer;
                            font-size: 12px;
                        "
                    >
                        <i class="fas fa-trash"></i>
                    </button>
                    <button 
                        class="log-toggle-btn"
                        title="Toggle collapse"
                        style="
                            padding: 4px 8px;
                            background: transparent;
                            border: none;
                            cursor: pointer;
                            font-size: 14px;
                            color: #666;
                        "
                    >
                        <i class=move || if is_collapsed.get() { "fas fa-chevron-up" } else { "fas fa-chevron-down" }></i>
                    </button>
                </div>
            </div>
            
            <div 
                class="log-content"
                style=move || format!("
                    overflow-y: auto;
                    flex: 1;
                    min-height: 0;
                    {}
                ", if is_collapsed.get() { "display: none;" } else { "display: block;" })
            >
                <div class="log-entries" style="padding: 8px;">
                    {move || {
                        refresh_trigger.get();
                        let entries = get_log_entries();
                        let mut reversed_entries = entries;
                        reversed_entries.reverse();
                        
                        reversed_entries.into_iter().enumerate().map(|(idx, entry)| {
                            let level_color = match entry.level.as_str() {
                                "ERROR" => "#dc3545",
                                "WARN" => "#ffc107", 
                                "INFO" => "#17a2b8",
                                "DEBUG" => "#6f42c1",
                                _ => "#6c757d",
                            };
                            
                            view! {
                                <div 
                                    key=idx
                                    class="log-entry" 
                                    style="
                                        display: flex;
                                        align-items: flex-start;
                                        gap: 8px;
                                        padding: 4px 8px;
                                        margin-bottom: 2px;
                                        border-radius: 4px;
                                        font-family: 'Courier New', monospace;
                                        font-size: 12px;
                                        line-height: 1.4;
                                        background: #f9f9f9;
                                    "
                                >
                                    <div class="log-timestamp" style="
                                        color: #666;
                                        min-width: 60px;
                                        font-size: 11px;
                                    ">{entry.timestamp}</div>
                                    <div class="log-level" style=format!("
                                        min-width: 50px;
                                        font-weight: bold;
                                        color: {};
                                    ", level_color)>
                                        {entry.level}
                                    </div>
                                    <div class="log-message" style="
                                        flex: 1;
                                        color: #333;
                                        word-break: break-word;
                                    ">{entry.message}</div>
                                </div>
                            }
                        }).collect::<Vec<_>>()
                    }}
                </div>
            </div>
        </div>
    }
} 