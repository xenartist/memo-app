use leptos::*;
use log;
use web_sys::{
    HtmlInputElement, 
    MouseEvent, 
    File, 
    FileReader,
    Event,
    Window,
    Document,
    ProgressEvent,
    SubmitEvent,
};
use crate::core::session::{Session, UserProfile, parse_user_profile};
use crate::core::rpc::RpcConnection;
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use image::{ImageBuffer, Luma};
use crate::core::pixel::Pixel;
use js_sys::Uint8Array;

#[component]
pub fn ProfilePage(
    session: RwSignal<Session>
) -> impl IntoView {
    let (user_profile, set_user_profile) = create_signal::<Option<UserProfile>>(None);

    // get cached profile from session (if there is any)
    if let Some(profile) = session.get().get_user_profile() {
        set_user_profile.set(Some(profile));
    }

    // after component mount, fetch data asynchronously
    create_effect(move |_| {
        // if there is cached data, don't need to request again
        if user_profile.get().is_some() {
            return;
        }

        spawn_local(async move {
            let mut current_session = session.get();
            if let Ok(pubkey) = current_session.get_public_key() {
                log::info!("Fetching profile for pubkey: {}", pubkey);
                
                let rpc = RpcConnection::new();
                match rpc.get_user_profile(&pubkey).await {
                    Ok(result) => {
                        if let Ok(profile) = parse_user_profile(&result) {
                            log::info!("Successfully fetched user profile");
                            current_session.set_user_profile(Some(profile.clone()));
                            session.set(current_session);
                            set_user_profile.set(Some(profile));
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to get user profile: {:?}", e);
                    }
                }
            }
        });
    });

    view! {
        <div class="profile-page">
            <h2>"User Profile"</h2>
            
            <div class="profile-content">
                {move || match user_profile.get() {
                    Some(profile) => view! {
                        <div class="profile-form">
                            <div class="form-group">
                                <label>"Username"</label>
                                <p class="profile-value">{profile.username}</p>
                            </div>
                            <div class="form-group">
                                <label>"Public Key"</label>
                                <p class="profile-value pubkey">{profile.pubkey}</p>
                            </div>
                            <div class="profile-stats">
                                <div class="stat-item">
                                    <label>"Total Minted"</label>
                                    <p class="stat-value">{profile.total_minted}</p>
                                </div>
                                <div class="stat-item">
                                    <label>"Total Burned"</label>
                                    <p class="stat-value">{profile.total_burned}</p>
                                </div>
                                <div class="stat-item">
                                    <label>"Mint Count"</label>
                                    <p class="stat-value">{profile.mint_count}</p>
                                </div>
                                <div class="stat-item">
                                    <label>"Burn Count"</label>
                                    <p class="stat-value">{profile.burn_count}</p>
                                </div>
                            </div>
                        </div>
                    },
                    None => view! {
                        <div class="profile-form">
                            <CreateProfileForm session=session/>
                        </div>
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn CreateProfileForm(
    session: RwSignal<Session>
) -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (pixel_art, set_pixel_art) = create_signal(Pixel::new());
    let (error_message, set_error_message) = create_signal(String::new());
    let (is_submitting, set_is_submitting) = create_signal(false);
    
    // Handle pixel click
    let handle_pixel_click = move |row: usize, col: usize| {
        let mut new_art = pixel_art.get();
        new_art.toggle_pixel(row, col);
        set_pixel_art.set(new_art);
    };

    // Handle image import
    let handle_import = move |ev: MouseEvent| {
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
        
        let onchange = Closure::wrap(Box::new(move |event: Event| {
            let input: HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
            if let Some(file) = input.files().unwrap().get(0) {
                let reader = FileReader::new().unwrap();
                let reader_clone = reader.clone();
                
                let onload = Closure::wrap(Box::new(move |e: ProgressEvent| {
                    if let Ok(buffer) = reader_clone.result() {
                        let array = Uint8Array::new(&buffer);
                        let data = array.to_vec();
                        
                        match Pixel::from_image_data(&data) {
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

    // Handle form submission
    let handle_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        
        // Validate inputs
        if username.get().is_empty() {
            set_error_message.set("Username is required".to_string());
            return;
        }
        if username.get().len() > 32 {
            set_error_message.set("Username too long. Maximum length is 32 characters".to_string());
            return;
        }

        // Get pixel art string representation
        let profile_image = pixel_art.get().to_optimal_string();

        // Validate profile image length
        if profile_image.len() > 256 {
            set_error_message.set("Profile image too long. Maximum length is 256 characters".to_string());
            return;
        }

        // Clear any previous error
        set_error_message.set(String::new());
        set_is_submitting.set(true);

        // Create RPC connection
        let rpc = RpcConnection::new();
        let username_clone = username.get().clone();
        let profile_image_clone = profile_image.clone();
        
        spawn_local(async move {
            // Get pubkey from session
            match session.get().get_public_key() {
                Ok(pubkey) => {
                    log::info!("Creating profile for pubkey: {}", pubkey);
                    match rpc.initialize_user_profile(
                        &pubkey,
                        &username_clone,
                        &profile_image_clone
                    ).await {
                        Ok(signature) => {
                            log::info!("Profile created successfully! Signature: {}", signature);
                            set_error_message.set("Profile created successfully!".to_string());
                            
                            // Refresh the page after successful creation
                            if let Some(window) = web_sys::window() {
                                if let Err(e) = window.location().reload() {
                                    log::error!("Failed to reload page: {:?}", e);
                                }
                            }
                        },
                        Err(e) => {
                            log::error!("Failed to create profile: {:?}", e);
                            set_error_message.set(format!("Failed to create profile: {}", e));
                        }
                    }
                },
                Err(e) => {
                    log::error!("Failed to get public key: {:?}", e);
                    set_error_message.set("Failed to get public key from session".to_string());
                }
            }
            set_is_submitting.set(false);
        });
    };

    view! {
        <form class="create-profile-form" on:submit=handle_submit>
            <h3>"Create Your Profile"</h3>
            
            <div class="form-group">
                <label for="username">"Username"</label>
                <input 
                    type="text"
                    id="username"
                    maxlength="32"
                    placeholder="Enter your username (max 32 characters)"
                    on:input=move |ev| {
                        let input = event_target::<HtmlInputElement>(&ev);
                        set_username.set(input.value());
                    }
                    prop:value=username
                    prop:disabled=is_submitting
                />
            </div>

            <div class="pixel-art-editor">
                <label>"Profile Image (32x32 Pixel Art)"</label>
                <div class="pixel-grid">
                    {move || {
                        let art = pixel_art.get();
                        let (rows, cols) = art.dimensions();
                        (0..rows).map(|row| {
                            view! {
                                <div class="pixel-row">
                                    {(0..cols).map(|col| {
                                        let is_black = art.get_pixel(row, col);
                                        view! {
                                            <div 
                                                class="pixel"
                                                class:black=is_black
                                                on:click=move |_| handle_pixel_click(row, col)
                                            />
                                        }
                                    }).collect_view()}
                                </div>
                            }
                        }).collect_view()
                    }}
                </div>

                <button 
                    type="button"
                    class="import-btn"
                    on:click=handle_import
                    prop:disabled=is_submitting
                >
                    "Import Image"
                </button>
            </div>

            <div class="error-message" class:success=move || error_message.get().contains("success")>
                {move || error_message.get()}
            </div>

            <button 
                type="submit" 
                class="submit-btn"
                prop:disabled=is_submitting
            >
                {move || if is_submitting.get() { "Creating Profile..." } else { "Create Profile" }}
            </button>
        </form>
    }
}

// Helper function to process imported image
async fn process_image(data: &[u8]) -> Result<Pixel, String> {
    let img = image::load_from_memory(data)
        .map_err(|e| format!("Failed to load image: {}", e))?;
    
    let resized = img.resize_exact(32, 32, image::imageops::FilterType::Lanczos3);
    let gray = resized.into_luma8();
    
    let threshold = 128u8;
    let mut pixel_art = Pixel::new();
    
    for (x, y, pixel) in gray.enumerate_pixels() {
        pixel_art.set_pixels_from_image(x as usize, y as usize, pixel[0] < threshold);
    }
    
    Ok(pixel_art)
} 