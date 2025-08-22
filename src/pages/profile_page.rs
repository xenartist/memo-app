use leptos::*;
use crate::core::session::Session;
use crate::core::rpc_profile::UserProfile;
use crate::pages::pixel_view::PixelView;
use crate::pages::memo_card::LazyPixelView;
use crate::core::pixel::Pixel;
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, File, FileReader, Event, ProgressEvent};
use wasm_bindgen::closure::Closure;
use js_sys::Uint8Array;
use std::rc::Rc;

#[component]
pub fn ProfilePage(session: RwSignal<Session>) -> impl IntoView {
    // Profile state
    let profile = create_rw_signal::<Option<UserProfile>>(None);
    let loading = create_rw_signal(false);
    let error_message = create_rw_signal::<Option<String>>(None);
    let success_message = create_rw_signal::<Option<String>>(None);
    
    // Form states
    let show_create_form = create_rw_signal(false);
    let show_edit_form = create_rw_signal(false);
    
    // Form fields
    let username = create_rw_signal(String::new());
    let about_me = create_rw_signal(String::new());
    let pixel_art = create_rw_signal(Pixel::new_with_size(16)); // default 16x16 pixel art
    let burn_amount = create_rw_signal(420u64); // Default minimum burn amount
    
    // Pixel art editor state
    let grid_size = create_rw_signal(16usize);
    let show_copied = create_rw_signal(false);
    
    // Load profile on page load
    create_effect(move |_| {
        let current_profile = session.with(|s| s.get_user_profile());
        profile.set(current_profile);
    });
    
    // Clear messages after 5 seconds
    let clear_messages = move || {
        set_timeout(
            move || {
                error_message.set(None);
                success_message.set(None);
            },
            std::time::Duration::from_secs(5),
        );
    };

    // Handle pixel art import
    let handle_import = move |_| {
        let document = web_sys::window().unwrap().document().unwrap();
        let input: HtmlInputElement = document
            .create_element("input")
            .unwrap()
            .dyn_into()
            .unwrap();
        
        input.set_type("file");
        input.set_accept("image/*");
        
        let pixel_art_write = pixel_art;
        let error_signal = error_message;
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
                                error_signal.set(None);
                            }
                            Err(e) => {
                                error_signal.set(Some(format!("Failed to process image: {}", e)));
                            }
                        }
                    }
                }) as Box<dyn FnMut(_)>);
                
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
    let copy_string = move |_| {
        let art_string = pixel_art.get().to_optimal_string();
        
        if let Some(window) = web_sys::window() {
            // Fix: navigator().clipboard() returns Clipboard directly, not Option<Clipboard>
            let navigator = window.navigator();
            let clipboard = navigator.clipboard();
            let _ = clipboard.write_text(&art_string);
            show_copied.set(true);
            
            set_timeout(
                move || show_copied.set(false),
                std::time::Duration::from_millis(1500),
            );
        }
    };
    
    // Create profile action
    let create_profile = create_action(move |_: &()| async move {
        loading.set(true);
        error_message.set(None);
        success_message.set(None);
        
        let username_val = username.get();
        let image_val = pixel_art.get().to_optimal_string(); // Use pixel art string
        let about_val = if about_me.get().is_empty() { None } else { Some(about_me.get()) };
        let burn_val = burn_amount.get(); // remove unit conversion, keep as tokens amount
        
        // Validate inputs
        if username_val.is_empty() {
            error_message.set(Some("Username is required".to_string()));
            loading.set(false);
            clear_messages();
            return;
        }
        
        if username_val.len() > 32 {
            error_message.set(Some("Username must be 32 characters or less".to_string()));
            loading.set(false);
            clear_messages();
            return;
        }
        
        if image_val.len() > 256 {
            error_message.set(Some("Pixel art string too long (max 256 characters)".to_string()));
            loading.set(false);
            clear_messages();
            return;
        }
        
        if let Some(ref about) = about_val {
            if about.len() > 128 {
                error_message.set(Some("About me must be 128 characters or less".to_string()));
                loading.set(false);
                clear_messages();
                return;
            }
        }
        
        match session.with_untracked(|s| s.clone()).create_profile(
            burn_val, // now passing 420 tokens instead of 420,000,000 units
            username_val,
            image_val,
            about_val,
        ).await {
            Ok(_) => {
                success_message.set(Some("Profile created successfully! Loading profile...".to_string()));
                show_create_form.set(false);
                
                // clear form
                username.set(String::new());
                pixel_art.set(Pixel::new_with_size(16));
                about_me.set(String::new());
                
                // wait 10 seconds for blockchain state to update, then refresh user profile
                let session_clone = session.clone();
                let profile_clone = profile.clone();
                let success_message_clone = success_message.clone();
                
                spawn_local(async move {
                    // wait 10 seconds
                    log::info!("Waiting 10 seconds for blockchain state to update...");
                    
                    use gloo_timers::future::TimeoutFuture;
                    TimeoutFuture::new(10_000).await;
                    
                    // now get user profile
                    log::info!("Fetching updated user profile...");
                    let updated_profile = session_clone.with(|s| s.get_user_profile());
                    profile_clone.set(updated_profile);
                    
                    success_message_clone.set(Some("Profile created and loaded successfully!".to_string()));
                });
                
                clear_messages();
            },
            Err(e) => {
                error_message.set(Some(format!("Failed to create profile: {}", e)));
                clear_messages();
            }
        }
        
        loading.set(false);
    });
    
    // Update profile action
    let update_profile = create_action(move |_: &()| async move {
        loading.set(true);
        error_message.set(None);
        success_message.set(None);
        
        let username_val = if username.get().is_empty() { None } else { Some(username.get()) };
        let image_val = {
            let art_string = pixel_art.get().to_optimal_string();
            if art_string.is_empty() || art_string == Pixel::new_with_size(16).to_optimal_string() {
                None // No image update
            } else {
                Some(art_string)
            }
        };
        let about_val = if about_me.get().is_empty() { None } else { Some(about_me.get()) };
        let burn_val = burn_amount.get(); // remove unit conversion
        
        // Validate inputs
        if let Some(ref username_str) = username_val {
            if username_str.len() > 32 {
                error_message.set(Some("Username must be 32 characters or less".to_string()));
                loading.set(false);
                clear_messages();
                return;
            }
        }
        
        if let Some(ref image_str) = image_val {
            if image_str.len() > 256 {
                error_message.set(Some("Pixel art string too long (max 256 characters)".to_string()));
                loading.set(false);
                clear_messages();
                return;
            }
        }
        
        // fix: handle simple Option<String> instead of nested
        if let Some(ref about_str) = about_val {
            if about_str.len() > 128 {
                error_message.set(Some("About me must be 128 characters or less".to_string()));
                loading.set(false);
                clear_messages();
                return;
            }
        }
        
        // simplified call
        match session.with_untracked(|s| s.clone()).update_profile(
            burn_val,
            username_val,
            image_val,
            about_val, // now it's a simple Option<String>
        ).await {
            Ok(_) => {
                success_message.set(Some("Profile updated successfully! Loading updated profile...".to_string()));
                show_edit_form.set(false);
                
                // wait 10 seconds for blockchain state to update, then refresh user profile
                let session_clone = session.clone();
                let profile_clone = profile.clone();
                let success_message_clone = success_message.clone();
                
                spawn_local(async move {
                    // wait 10 seconds
                    log::info!("Waiting 10 seconds for blockchain state to update...");
                    
                    use gloo_timers::future::TimeoutFuture;
                    TimeoutFuture::new(10_000).await;
                    
                    // now get user profile
                    log::info!("Fetching updated user profile...");
                    let updated_profile = session_clone.with(|s| s.get_user_profile());
                    profile_clone.set(updated_profile);
                    
                    success_message_clone.set(Some("Profile updated and loaded successfully!".to_string()));
                });
                
                clear_messages();
            },
            Err(e) => {
                error_message.set(Some(format!("Failed to update profile: {}", e)));
                clear_messages();
            }
        }
        
        loading.set(false);
    });
    
    // Delete profile action
    let delete_profile = create_action(move |_: &()| async move {
        loading.set(true);
        error_message.set(None);
        success_message.set(None);
        
        match session.with_untracked(|s| s.clone()).delete_profile().await {
            Ok(_) => {
                success_message.set(Some("Profile deleted successfully!".to_string()));
                profile.set(None);
                clear_messages();
            },
            Err(e) => {
                error_message.set(Some(format!("Failed to delete profile: {}", e)));
                clear_messages();
            }
        }
        
        loading.set(false);
    });
    
    // Fill form with current profile data for editing
    let fill_edit_form = move || {
        if let Some(ref current_profile) = profile.get() {
            username.set(current_profile.username.clone());
            // use image field instead of profile_image
            if let Some(parsed_pixel) = Pixel::from_safe_string(&current_profile.image) {
                pixel_art.set(parsed_pixel);
            } else {
                // If not a valid pixel art string, create new empty pixel art
                pixel_art.set(Pixel::new_with_size(16));
            }
            // about_me is now Option<String>
            about_me.set(current_profile.about_me.clone().unwrap_or_default());
        }
        show_edit_form.set(true);
    };

    // Helper function to format timestamp (modified to handle i64)
    let format_timestamp = |timestamp: i64| -> String {
        let date = web_sys::js_sys::Date::new(&JsValue::from_f64(timestamp as f64 * 1000.0));
        date.to_locale_string("en-US", &web_sys::js_sys::Object::new()).as_string().unwrap_or_else(|| "Invalid Date".to_string())
    };
    
    view! {
        <div class="profile-page">
            <div class="container">
                <h1>
                    <i class="fas fa-user"></i>
                    "Profile Management"
                </h1>
                
                // Messages
                {move || error_message.get().map(|msg| view! {
                    <div class="alert alert-error">
                        <i class="fas fa-exclamation-triangle"></i>
                        {msg}
                    </div>
                })}
                
                {move || success_message.get().map(|msg| view! {
                    <div class="alert alert-success">
                        <i class="fas fa-check-circle"></i>
                        {msg}
                    </div>
                })}
                
                // Profile Display
                {move || match profile.get() {
                    Some(user_profile) => {
                        let created_str = format_timestamp(user_profile.created_at);
                        let updated_str = format_timestamp(user_profile.last_updated); // use last_updated
                        
                        view! {
                            <div class="profile-display">
                                <div class="profile-header">
                                    <div class="profile-info">
                                        <h2>
                                            <i class="fas fa-user"></i>
                                            {user_profile.username.clone()}
                                        </h2>
                                        <div class="profile-dates">
                                            <div class="date-item">
                                                <i class="fas fa-calendar-plus"></i>
                                                <span>"Created: " {created_str.clone()}</span>
                                            </div>
                                            <div class="date-item">
                                                <i class="fas fa-calendar-edit"></i>
                                                <span>"Updated: " {updated_str.clone()}</span>
                                            </div>
                                        </div>
                                    </div>
                                    
                                    {if !user_profile.image.is_empty() { // use image field
                                        // Check if it's a valid pixel art string
                                        if user_profile.image.starts_with("c:") || user_profile.image.starts_with("n:") {
                                            view! {
                                                <div class="profile-image">
                                                    <LazyPixelView
                                                        art={user_profile.image.clone()}
                                                        size=200
                                                    />
                                                </div>
                                            }.into_view()
                                        } else {
                                            view! {
                                                <div class="profile-image">
                                                    <img src={user_profile.image.clone()} alt="Profile Image" />
                                                </div>
                                            }.into_view()
                                        }
                                    } else {
                                        view! { 
                                            <div class="profile-image placeholder">
                                                <i class="fas fa-user-circle"></i>
                                            </div> 
                                        }.into_view()
                                    }}
                                </div>
                                
                                {if let Some(about_me_text) = &user_profile.about_me { // about_me is Option<String>
                                    view! {
                                        <div class="profile-about">
                                            <h3>
                                                <i class="fas fa-info-circle"></i>
                                                "About Me"
                                            </h3>
                                            <p>{about_me_text.clone()}</p>
                                        </div>
                                    }.into_view()
                                } else {
                                    view! { <span></span> }.into_view()
                                }}
                                
                                <div class="profile-meta">
                                    <p><strong>"User Address:"</strong> {user_profile.user.clone()}</p>
                                    <p><strong>"Created:"</strong> {created_str}</p>
                                    <p><strong>"Last Updated:"</strong> {updated_str}</p>
                                    <p><strong>"Bump:"</strong> {user_profile.bump.to_string()}</p>
                                </div>
                            </div>
                        }.into_view()
                    },
                    None => view! {
                        <div class="no-profile">
                            <div class="no-profile-card">
                                <i class="fas fa-user-plus" style="font-size: 3rem; color: #667eea; margin-bottom: 20px;"></i>
                                <h2>"No Profile Found"</h2>
                                <p>"You don't have a profile yet. Create one to get started!"</p>
                                <button 
                                    class="btn btn-primary"
                                    on:click=move |_| show_create_form.set(true)
                                    disabled=move || loading.get()
                                >
                                    <i class="fas fa-plus"></i>
                                    "Create Profile"
                                </button>
                            </div>
                        </div>
                    }.into_view()
                }}
                
                // Create Profile Form
                {move || show_create_form.get().then(|| view! {
                    <div class="profile-form">
                        <div class="form-card">
                            <h2>
                                <i class="fas fa-user-plus"></i>
                                "Create Profile"
                            </h2>
                            <form on:submit=move |e| {
                                e.prevent_default();
                                create_profile.dispatch(());
                            }>
                                <div class="form-group">
                                    <label for="username">
                                        <i class="fas fa-user"></i>
                                        "Username (required, max 32 characters)"
                                    </label>
                                    <input 
                                        type="text"
                                        id="username"
                                        prop:value=move || username.get()
                                        on:input=move |e| username.set(event_target_value(&e))
                                        maxlength="32"
                                        required
                                    />
                                </div>
                                
                                // Pixel Art Editor
                                <div class="form-group">
                                    <div class="pixel-art-editor">
                                        <div class="pixel-art-header">
                                            <label>
                                                <i class="fas fa-image"></i>
                                                "Profile Image (Pixel Art)"
                                            </label>
                                            <div class="pixel-art-controls">
                                                <select
                                                    class="size-selector"
                                                    prop:value=move || grid_size.get().to_string()
                                                    on:change=move |ev| {
                                                        let value = event_target_value(&ev);
                                                        if let Ok(size) = value.parse::<usize>() {
                                                            grid_size.set(size);
                                                            pixel_art.set(Pixel::new_with_size(size));
                                                        }
                                                    }
                                                    prop:disabled=move || loading.get()
                                                >
                                                    <option value="16">"16×16 pixels"</option>
                                                    <option value="32">"32×32 pixels"</option>
                                                </select>
                                                <button 
                                                    type="button"
                                                    class="import-btn"
                                                    on:click=handle_import
                                                    prop:disabled=move || loading.get()
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
                                                pixel_art.set(new_art);
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
                                </div>
                                
                                <div class="form-group">
                                    <label for="about-me">
                                        <i class="fas fa-info-circle"></i>
                                        "About Me"
                                    </label>
                                    <textarea 
                                        id="about-me"
                                        prop:value=move || about_me.get()
                                        on:input=move |e| about_me.set(event_target_value(&e))
                                        maxlength="128"
                                        rows="3"
                                        placeholder="Tell us about yourself..."
                                    ></textarea>
                                    <div class="char-count">
                                        {move || format!("{}/128", about_me.get().len())}
                                    </div>
                                </div>
                                
                                <div class="form-group">
                                    <label for="burn-amount">
                                        <i class="fas fa-fire"></i>
                                        "Burn Amount (tokens, minimum 420)"
                                    </label>
                                    <input 
                                        type="number"
                                        id="burn-amount"
                                        prop:value=move || burn_amount.get()
                                        on:input=move |e| {
                                            if let Ok(val) = event_target_value(&e).parse::<u64>() {
                                                burn_amount.set(val.max(420));
                                            }
                                        }
                                        min="420"
                                        required
                                    />
                                </div>
                                
                                <div class="form-actions">
                                    <button 
                                        type="submit"
                                        class="btn btn-primary"
                                        disabled=move || loading.get()
                                    >
                                        <i class="fas fa-check"></i>
                                        {move || if loading.get() { "Creating..." } else { "Create Profile" }}
                                    </button>
                                    <button 
                                        type="button"
                                        class="btn btn-secondary"
                                        on:click=move |_| show_create_form.set(false)
                                        disabled=move || loading.get()
                                    >
                                        <i class="fas fa-times"></i>
                                        "Cancel"
                                    </button>
                                </div>
                            </form>
                        </div>
                    </div>
                })}
                
                // Edit Profile Form
                {move || show_edit_form.get().then(|| view! {
                    <div class="profile-form">
                        <div class="form-card">
                            <h2>
                                <i class="fas fa-edit"></i>
                                "Edit Profile"
                            </h2>
                            <form on:submit=move |e| {
                                e.prevent_default();
                                update_profile.dispatch(());
                            }>
                                <div class="form-group">
                                    <label for="edit-username">
                                        <i class="fas fa-user"></i>
                                        "Username (leave empty to keep current, max 32 characters)"
                                    </label>
                                    <input 
                                        type="text"
                                        id="edit-username"
                                        prop:value=move || username.get()
                                        on:input=move |e| username.set(event_target_value(&e))
                                        maxlength="32"
                                    />
                                </div>
                                
                                // Pixel Art Editor (same as create form)
                                <div class="form-group">
                                    <div class="pixel-art-editor">
                                        <div class="pixel-art-header">
                                            <label>
                                                <i class="fas fa-image"></i>
                                                "Profile Image (Pixel Art) - leave empty to keep current"
                                            </label>
                                            <div class="pixel-art-controls">
                                                <select
                                                    class="size-selector"
                                                    prop:value=move || grid_size.get().to_string()
                                                    on:change=move |ev| {
                                                        let value = event_target_value(&ev);
                                                        if let Ok(size) = value.parse::<usize>() {
                                                            grid_size.set(size);
                                                            pixel_art.set(Pixel::new_with_size(size));
                                                        }
                                                    }
                                                    prop:disabled=move || loading.get()
                                                >
                                                    <option value="16">"16×16 pixels"</option>
                                                    <option value="32">"32×32 pixels"</option>
                                                </select>
                                                <button 
                                                    type="button"
                                                    class="import-btn"
                                                    on:click=handle_import
                                                    prop:disabled=move || loading.get()
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
                                                pixel_art.set(new_art);
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
                                </div>
                                
                                <div class="form-group">
                                    <label for="edit-about-me">
                                        <i class="fas fa-info-circle"></i>
                                        "About Me (leave empty to keep current, max 128 characters)"
                                    </label>
                                    <textarea 
                                        id="edit-about-me"
                                        prop:value=move || about_me.get()
                                        on:input=move |e| about_me.set(event_target_value(&e))
                                        maxlength="128"
                                        rows="3"
                                        placeholder="Tell us about yourself..."
                                    ></textarea>
                                    <div class="char-count">
                                        {move || format!("{}/128", about_me.get().len())}
                                    </div>
                                </div>
                                
                                <div class="form-group">
                                    <label for="edit-burn-amount">
                                        <i class="fas fa-fire"></i>
                                        "Burn Amount (tokens, minimum 420)"
                                    </label>
                                    <input 
                                        type="number"
                                        id="edit-burn-amount"
                                        prop:value=move || burn_amount.get()
                                        on:input=move |e| {
                                            if let Ok(val) = event_target_value(&e).parse::<u64>() {
                                                burn_amount.set(val.max(420));
                                            }
                                        }
                                        min="420"
                                        required
                                    />
                                </div>
                                
                                <div class="form-actions">
                                    <button 
                                        type="submit"
                                        class="btn btn-primary"
                                        disabled=move || loading.get()
                                    >
                                        <i class="fas fa-save"></i>
                                        {move || if loading.get() { "Updating..." } else { "Update Profile" }}
                                    </button>
                                    <button 
                                        type="button"
                                        class="btn btn-secondary"
                                        on:click=move |_| show_edit_form.set(false)
                                        disabled=move || loading.get()
                                    >
                                        <i class="fas fa-times"></i>
                                        "Cancel"
                                    </button>
                                </div>
                            </form>
                        </div>
                    </div>
                })}
            </div>
        </div>
    }
} 