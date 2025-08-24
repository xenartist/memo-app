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
    let show_delete_confirm = create_rw_signal(false);
    
    // add countdown state
    let countdown_seconds = create_rw_signal(0i32);
    let is_waiting_for_blockchain = create_rw_signal(false);
    
    // Form fields
    let username = create_rw_signal(String::new());
    let about_me = create_rw_signal(String::new());
    let pixel_art = create_rw_signal(Pixel::new_with_size(32)); // fixed size 32x32
    let burn_amount = create_rw_signal(420u64); // Default minimum burn amount
    
    // Original values for change detection
    let original_username = create_rw_signal(String::new());
    let original_about_me = create_rw_signal(String::new());
    let original_pixel_art = create_rw_signal(Pixel::new_with_size(32)); // fixed size 32x32
    
    // Pixel art editor state - remove grid_size and current_pixel_size, because fixed size 32x32
    let show_copied = create_rw_signal(false);
    
    // Change detection signals
    let username_changed = create_memo(move |_| username.get() != original_username.get());
    let about_me_changed = create_memo(move |_| about_me.get() != original_about_me.get());
    let pixel_art_changed = create_memo(move |_| {
        pixel_art.get().to_optimal_string() != original_pixel_art.get().to_optimal_string()
    });
    let has_changes = create_memo(move |_| {
        username_changed.get() || about_me_changed.get() || pixel_art_changed.get()
    });
    
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

    // Handle pixel art import - fixed size 32x32
    let handle_import = move |_| {
        log::info!("Starting image import...");
        
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
        
        let onchange = Closure::wrap(Box::new(move |event: Event| {
            log::info!("File selected...");
            let input: HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
            if let Some(file) = input.files().unwrap().get(0) {
                log::info!("Processing file: {}", file.name());
                let reader = FileReader::new().unwrap();
                let reader_clone = reader.clone();
                
                // Clone the signals for use in the closure
                let pixel_art_clone = pixel_art_write;
                let error_clone = error_signal;
                
                let onload = Closure::wrap(Box::new(move |_: ProgressEvent| {
                    log::info!("File read complete, processing image...");
                    if let Ok(buffer) = reader_clone.result() {
                        let array = Uint8Array::new(&buffer);
                        let data = array.to_vec();
                        log::info!("Image data size: {} bytes", data.len());
                        
                        // Fixed size 32x32
                        match Pixel::from_image_data_with_size(&data, 32) {
                            Ok(new_art) => {
                                log::info!("Successfully created pixel art from image");
                                pixel_art_clone.set(new_art);
                                error_clone.set(None);
                            }
                            Err(e) => {
                                log::error!("Failed to process image: {}", e);
                                error_clone.set(Some(format!("Failed to process image: {}", e)));
                            }
                        }
                    } else {
                        log::error!("Failed to read file buffer");
                        error_clone.set(Some("Failed to read file".to_string()));
                    }
                }) as Box<dyn FnMut(_)>);
                
                let onerror = Closure::wrap(Box::new(move |_: Event| {
                    log::error!("FileReader error occurred");
                    error_signal.set(Some("Failed to read file".to_string()));
                }) as Box<dyn FnMut(_)>);
                
                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                reader.set_onerror(Some(onerror.as_ref().unchecked_ref()));
                onload.forget();
                onerror.forget();
                
                reader.read_as_array_buffer(&file).unwrap();
            } else {
                log::warn!("No file selected");
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
        
        // Give UI time to update the loading state
        use gloo_timers::future::TimeoutFuture;
        TimeoutFuture::new(100).await;
        
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
                pixel_art.set(Pixel::new_with_size(32));
                about_me.set(String::new());
                
                // start countdown
                is_waiting_for_blockchain.set(true);
                countdown_seconds.set(20);
                
                // don't call clear_messages immediately, wait for countdown to end
                
                // wait 20 seconds for blockchain state to update, then refresh user profile
                let session_clone = session.clone();
                let profile_clone = profile.clone();
                let success_message_clone = success_message.clone();
                let countdown_clone = countdown_seconds.clone();
                let waiting_clone = is_waiting_for_blockchain.clone();
                let error_message_clone = error_message.clone();
                
                spawn_local(async move {
                    // countdown loop
                    for i in (1..=20).rev() {
                        countdown_clone.set(i);
                        TimeoutFuture::new(1_000).await; // wait 1 second
                    }
                    
                    countdown_clone.set(0);
                    
                    // fetch user profile from blockchain, not from cache
                    log::info!("Fetching updated user profile from blockchain...");
                    match session_clone.with_untracked(|s| s.clone()).fetch_and_cache_user_profile().await {
                        Ok(Some(updated_profile)) => {
                            profile_clone.set(Some(updated_profile));
                            success_message_clone.set(Some("Profile created and loaded successfully!".to_string()));
                            waiting_clone.set(false);
                            
                            // call clear_messages
                            set_timeout(
                                move || {
                                    error_message_clone.set(None);
                                    success_message_clone.set(None);
                                },
                                std::time::Duration::from_secs(5),
                            );
                        },
                        Ok(None) => {
                            log::warn!("Profile still not found after creation, retrying...");
                            success_message_clone.set(Some("Still loading... Please wait a moment more.".to_string()));
                            
                            // show retry countdown
                            countdown_clone.set(5);
                            for i in (1..=5).rev() {
                                countdown_clone.set(i);
                                TimeoutFuture::new(1_000).await;
                            }
                            
                            // if still not found, wait 5 seconds and retry
                            match session_clone.with_untracked(|s| s.clone()).fetch_and_cache_user_profile().await {
                                Ok(Some(retry_profile)) => {
                                    profile_clone.set(Some(retry_profile));
                                    success_message_clone.set(Some("Profile created and loaded successfully!".to_string()));
                                },
                                _ => {
                                    success_message_clone.set(Some("Profile created successfully! Please refresh the page if it doesn't appear.".to_string()));
                                }
                            }
                            countdown_clone.set(0);
                            waiting_clone.set(false);
                        },
                        Err(e) => {
                            log::error!("Failed to fetch updated profile: {}", e);
                            success_message_clone.set(Some("Profile created successfully! Please refresh the page if it doesn't appear.".to_string()));
                            countdown_clone.set(0);
                            waiting_clone.set(false);
                        }
                    }
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
        
        // Always send complete profile data, not just changes
        let username_val = Some(username.get());
        let image_val = Some(pixel_art.get().to_optimal_string());
        let about_val = if about_me.get().is_empty() { None } else { Some(about_me.get()) };
        let burn_val = burn_amount.get();
        
        // Validate inputs
        if username.get().len() > 32 {
            error_message.set(Some("Username must be 32 characters or less".to_string()));
            loading.set(false);
            clear_messages();
            return;
        }
        
        if pixel_art.get().to_optimal_string().len() > 256 {
            error_message.set(Some("Pixel art string too long (max 256 characters)".to_string()));
            loading.set(false);
            clear_messages();
            return;
        }
        
        if let Some(ref about_str) = about_val {
            if about_str.len() > 128 {
                error_message.set(Some("About me must be 128 characters or less".to_string()));
                loading.set(false);
                clear_messages();
                return;
            }
        }
        
        // Give UI time to update the loading state
        use gloo_timers::future::TimeoutFuture;
        TimeoutFuture::new(100).await;
        
        match session.with_untracked(|s| s.clone()).update_profile(
            burn_val,
            username_val,
            image_val,
            about_val,
        ).await {
            Ok(_) => {
                success_message.set(Some("Profile updated successfully! Loading updated profile...".to_string()));
                show_edit_form.set(false);
                
                // wait 20 seconds for blockchain state to update, then refresh user profile
                let session_clone = session.clone();
                let profile_clone = profile.clone();
                let success_message_clone = success_message.clone();
                
                spawn_local(async move {
                    // wait 20 seconds (shorten the waiting time, because session has already tried to get once in update_profile)
                    log::info!("Waiting 20 seconds for blockchain state to update...");
                    
                    TimeoutFuture::new(20_000).await;
                    
                    // re-get user profile from blockchain, not from cache
                    log::info!("Fetching updated user profile...");
                    match session_clone.with_untracked(|s| s.clone()).fetch_and_cache_user_profile().await {
                        Ok(Some(updated_profile)) => {
                            profile_clone.set(Some(updated_profile));
                            success_message_clone.set(Some("Profile updated and loaded successfully!".to_string()));
                        },
                        Ok(None) => {
                            // Profile not found, clear the cache in update_profile
                            profile_clone.set(None);
                            success_message_clone.set(Some("Profile updated successfully!".to_string()));
                        },
                        Err(e) => {
                            log::error!("Failed to fetch updated profile: {}", e);
                            // if failed to get, at least get from cache in update_profile
                            let cached_profile = session_clone.with(|s| s.get_user_profile());
                            profile_clone.set(cached_profile);
                            success_message_clone.set(Some("Profile updated successfully! (using cached data)".to_string()));
                        }
                    }
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
        
        // Give UI time to update the loading state
        use gloo_timers::future::TimeoutFuture;
        TimeoutFuture::new(100).await;
        
        match session.with_untracked(|s| s.clone()).delete_profile().await {
            Ok(_) => {
                success_message.set(Some("Profile deleted successfully!".to_string()));
                show_delete_confirm.set(false);
                
                // clear profile cache and refresh in delete_profile
                session.update(|s| s.set_user_profile(None));
                profile.set(None);
                
                clear_messages();
            },
            Err(e) => {
                error_message.set(Some(format!("Failed to delete profile: {}", e)));
                show_delete_confirm.set(false);
                clear_messages();
            }
        }
        
        loading.set(false);
    });
    
    // Fill form with current profile data for editing
    let fill_edit_form = move || {
        if let Some(ref current_profile) = profile.get() {
            // Set current values
            username.set(current_profile.username.clone());
            
            // Set pixel art - always use 32x32
            if let Some(parsed_pixel) = Pixel::from_optimal_string(&current_profile.image) {
                log::info!("Successfully parsed pixel art from string");
                pixel_art.set(parsed_pixel.clone());
                original_pixel_art.set(parsed_pixel);
            } else {
                log::warn!("Failed to parse pixel art from string, using empty 32x32: {}", current_profile.image);
                // always create 32x32 pixel art
                pixel_art.set(Pixel::new_with_size(32));
                original_pixel_art.set(Pixel::new_with_size(32));
            }
            
            // Set about me
            let about_text = current_profile.about_me.clone().unwrap_or_default();
            about_me.set(about_text.clone());
            
            // Store original values for change detection
            original_username.set(current_profile.username.clone());
            original_about_me.set(about_text);
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
            <div class={move || if profile.get().is_none() { "container no-profile-container" } else { "container" }}>
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
                
                // add independent countdown display after success message
                {move || if is_waiting_for_blockchain.get() && countdown_seconds.get() > 0 {
                    view! {
                        <div class="alert alert-info">
                            <div class="countdown-display">
                                <div class="countdown-progress">
                                    <i class="fas fa-clock"></i>
                                    "Loading from blockchain... " 
                                    <span class="countdown-number">{countdown_seconds.get()}</span>
                                    " seconds remaining"
                                </div>
                                <div class="progress-bar">
                                    <div 
                                        class="progress-fill"
                                        style=move || format!("width: {}%", ((20 - countdown_seconds.get()) * 100 / 20))
                                    ></div>
                                </div>
                            </div>
                        </div>
                    }.into_view()
                } else {
                    view! { <span></span> }.into_view()
                }}
                
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
                                
                                // action buttons area
                                <div class="profile-actions">
                                    <button 
                                        class="btn btn-primary"
                                        on:click=move |_| {
                                            fill_edit_form();
                                        }
                                        disabled=move || loading.get()
                                    >
                                        <i class="fas fa-edit"></i>
                                        "Update Profile"
                                    </button>
                                    
                                    <button 
                                        class="btn btn-danger"
                                        on:click=move |_| {
                                            show_delete_confirm.set(true);
                                        }
                                        disabled=move || loading.get()
                                    >
                                        <i class="fas fa-trash"></i>
                                        "Delete Profile"
                                    </button>
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
                    <div class="modal-overlay">
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
                                        on:input=move |e| {
                                            username.set(event_target_value(&e));
                                            original_username.set(username.get());
                                        }
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
                                                "Profile Image (Pixel Art - 32×32)"
                                            </label>
                                            <div class="pixel-art-controls">
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
                                        
                                        // Pixel Art Canvas - fixed size 32x32
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
                                        "About Me (max 128 characters)"
                                    </label>
                                    <textarea 
                                        id="about-me"
                                        prop:value=move || about_me.get()
                                        on:input=move |e| {
                                            about_me.set(event_target_value(&e));
                                            original_about_me.set(about_me.get());
                                        }
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
                    <div class="modal-overlay">
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
                                        "Username (max 32 characters)"
                                        {move || if username_changed.get() {
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
                                        id="edit-username"
                                        prop:value=move || username.get()
                                        on:input=move |e| username.set(event_target_value(&e))
                                        maxlength="32"
                                        class:changed=move || username_changed.get()
                                        required
                                    />
                                </div>
                                
                                // Pixel Art Editor
                                <div class="form-group">
                                    <div class="pixel-art-editor">
                                        <div class="pixel-art-header">
                                            <label>
                                                <i class="fas fa-image"></i>
                                                "Profile Image (Pixel Art - 32×32)"
                                                {move || if pixel_art_changed.get() {
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
                                        
                                        // Pixel Art Canvas - fixed size 32x32
                                        <div class:changed=move || pixel_art_changed.get()>
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
                                        </div>

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
                                        "About Me (max 128 characters)"
                                        {move || if about_me_changed.get() {
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
                                        id="edit-about-me"
                                        prop:value=move || about_me.get()
                                        on:input=move |e| about_me.set(event_target_value(&e))
                                        maxlength="128"
                                        rows="3"
                                        placeholder="Tell us about yourself..."
                                        class:changed=move || about_me_changed.get()
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
                                
                                // Changes summary
                                {move || if has_changes.get() {
                                    view! {
                                        <div class="changes-summary">
                                            <h4>
                                                <i class="fas fa-exclamation-circle"></i>
                                                "Pending Changes"
                                            </h4>
                                            <ul>
                                                {move || if username_changed.get() {
                                                    view! {
                                                        <li>
                                                            "Username: "
                                                            <span class="old-value">{original_username.get()}</span>
                                                            " → "
                                                            <span class="new-value">{username.get()}</span>
                                                        </li>
                                                    }.into_view()
                                                } else {
                                                    view! { <span></span> }.into_view()
                                                }}
                                                
                                                {move || if about_me_changed.get() {
                                                    view! {
                                                        <li>
                                                            "About Me: "
                                                            <span class="old-value">
                                                                {if original_about_me.get().is_empty() { 
                                                                    "(empty)".to_string() 
                                                                } else { 
                                                                    original_about_me.get() 
                                                                }}
                                                            </span>
                                                            " → "
                                                            <span class="new-value">
                                                                {if about_me.get().is_empty() { 
                                                                    "(empty)".to_string() 
                                                                } else { 
                                                                    about_me.get() 
                                                                }}
                                                            </span>
                                                        </li>
                                                    }.into_view()
                                                } else {
                                                    view! { <span></span> }.into_view()
                                                }}
                                                
                                                {move || if pixel_art_changed.get() {
                                                    view! {
                                                        <li>
                                                            "Pixel Art: Modified"
                                                        </li>
                                                    }.into_view()
                                                } else {
                                                    view! { <span></span> }.into_view()
                                                }}
                                            </ul>
                                        </div>
                                    }.into_view()
                                } else {
                                    view! { <span></span> }.into_view()
                                }}
                                
                                <div class="form-actions">
                                    <button 
                                        type="submit"
                                        class="btn btn-primary"
                                        disabled=move || loading.get() || !has_changes.get()
                                    >
                                        <i class="fas fa-save"></i>
                                        {move || if loading.get() { 
                                            "Updating..." 
                                        } else if !has_changes.get() {
                                            "No Changes to Save"
                                        } else { 
                                            "Update Profile" 
                                        }}
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
        
        // delete confirm dialog
        {move || if show_delete_confirm.get() {
            view! {
                <div class="modal-overlay">
                    <div class="modal-content">
                        {move || if loading.get() {
                            // deleting status
                            view! {
                                <div class="modal-header">
                                    <h3>
                                        <i class="fas fa-spinner fa-spin"></i>
                                        "Deleting Profile"
                                    </h3>
                                </div>
                                
                                <div class="modal-body">
                                    <div class="deleting-status">
                                        <div class="loading-spinner"></div>
                                        <p>"Please wait while we delete your profile from the blockchain..."</p>
                                        <p class="warning-text">
                                            <i class="fas fa-clock"></i>
                                            "This may take a few moments."
                                        </p>
                                    </div>
                                </div>
                            }.into_view()
                        } else {
                            // confirm delete status
                            view! {
                                <div class="modal-header">
                                    <h3>
                                        <i class="fas fa-exclamation-triangle"></i>
                                        "Confirm Delete Profile"
                                    </h3>
                                </div>
                                
                                <div class="modal-body">
                                    <p><strong>"Warning:"</strong> " This action cannot be undone!"</p>
                                    <p>"Are you sure you want to permanently delete your profile?"</p>
                                    <p class="delete-info">
                                        <i class="fas fa-info-circle"></i>
                                        "Your profile data will be removed from the blockchain and cannot be recovered."
                                    </p>
                                </div>
                                
                                <div class="modal-actions">
                                    <button 
                                        class="btn btn-danger"
                                        on:click=move |_| {
                                            delete_profile.dispatch(());
                                        }
                                    >
                                        <i class="fas fa-trash"></i>
                                        "Yes, Delete Profile"
                                    </button>
                                    
                                    <button 
                                        class="btn btn-secondary"
                                        on:click=move |_| {
                                            show_delete_confirm.set(false);
                                        }
                                    >
                                        <i class="fas fa-times"></i>
                                        "Cancel"
                                    </button>
                                </div>
                            }.into_view()
                        }}
                    </div>
                </div>
            }.into_view()
        } else {
            view! { <span></span> }.into_view()
        }}
    }
} 