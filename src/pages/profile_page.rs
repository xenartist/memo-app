use leptos::*;
use log;
use web_sys::{HtmlInputElement, MouseEvent, File, FileReader};
use crate::core::session::{Session, UserProfile, parse_user_profile};
use crate::core::rpc::RpcConnection;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use image::{ImageBuffer, Luma};
use tauri_plugin_fs::FsExt;

// pixel art data type
#[derive(Clone)]
struct PixelArt {
    pixels: Vec<Vec<bool>>, // true represents black, false represents white
}

impl PixelArt {
    fn new() -> Self {
        // create 32x32 blank canvas
        Self {
            pixels: vec![vec![false; 32]; 32]
        }
    }

    // Convert pixel art to hex string
    fn to_hex_string(&self) -> String {
        let mut binary_string = String::with_capacity(1024);
        
        // Convert 2D array to binary string
        for row in &self.pixels {
            for &pixel in row {
                binary_string.push(if pixel { '1' } else { '0' });
            }
        }
        
        // Convert binary string to hex string
        let mut hex_string = String::with_capacity(256);
        for chunk in binary_string.as_bytes().chunks(4) {
            let mut value = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                if bit == b'1' {
                    value |= 1 << (3 - i);
                }
            }
            hex_string.push_str(&format!("{:X}", value));
        }
        
        hex_string
    }

    // Create from hex string
    fn from_hex_string(hex_string: &str) -> Option<Self> {
        if hex_string.len() != 256 || !hex_string.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }

        let mut pixels = vec![vec![false; 32]; 32];
        let mut pixel_index = 0;

        for hex_char in hex_string.chars() {
            let value = hex_char.to_digit(16)?;
            let binary = format!("{:04b}", value);
            
            for bit in binary.chars() {
                let row = pixel_index / 32;
                let col = pixel_index % 32;
                pixels[row][col] = bit == '1';
                pixel_index += 1;
            }
        }

        Some(Self { pixels })
    }
}

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
                            <CreateProfileForm />
                        </div>
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn CreateProfileForm() -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (pixel_art, set_pixel_art) = create_signal(PixelArt::new());
    let (error_message, set_error_message) = create_signal(String::new());
    
    // Handle pixel click
    let handle_pixel_click = move |row: usize, col: usize| {
        let mut new_art = pixel_art.get();
        new_art.pixels[row][col] = !new_art.pixels[row][col];
        set_pixel_art.set(new_art);
    };

    // Handle image import
    let handle_import = move |_| {
        spawn_local(async move {
            // open file selection dialog
            let file_path = match tauri_plugin_fs::dialog::FileDialogBuilder::new()
                .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp"])
                .pick_file()
                .await
            {
                Some(path) => path,
                None => return, // user cancelled the selection
            };

            // read file content
            match tauri_plugin_fs::read_binary(&file_path).await {
                Ok(data) => {
                    match process_image(&data) {
                        Ok(new_art) => {
                            pixel_art.set(new_art);
                            set_error_message.set(String::new());
                        }
                        Err(e) => {
                            set_error_message.set(format!("Failed to process image: {}", e));
                        }
                    }
                }
                Err(e) => {
                    set_error_message.set(format!("Failed to read file: {}", e));
                }
            }
        });
    };

    // Handle form submission
    let handle_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        
        let username_value = username.get();
        if username_value.is_empty() {
            set_error_message.set("Username is required".to_string());
            return;
        }
        
        if username_value.len() > 32 {
            set_error_message.set("Username must be at most 32 characters".to_string());
            return;
        }
        
        let profile_image = pixel_art.get().to_hex_string();
        
        // Get pubkey from session
        let current_session = session.get();
        match current_session.get_public_key() {
            Ok(pubkey) => {
                spawn_local(async move {
                    let rpc = RpcConnection::new();
                    match rpc.initialize_user_profile(&pubkey, &username_value, &profile_image).await {
                        Ok(_) => {
                            // Handle success
                            log::info!("Profile created successfully");
                        }
                        Err(e) => {
                            set_error_message.set(format!("Failed to create profile: {}", e));
                        }
                    }
                });
            }
            Err(e) => {
                set_error_message.set(format!("Failed to get public key: {}", e));
            }
        }
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
                />
            </div>

            <div class="pixel-art-editor">
                <label>"Profile Image (32x32 Pixel Art)"</label>
                <div class="pixel-grid">
                    {move || {
                        pixel_art.get().pixels.iter().enumerate().map(|(row_idx, row)| {
                            view! {
                                <div class="pixel-row">
                                    {row.iter().enumerate().map(|(col_idx, &is_black)| {
                                        let row = row_idx;
                                        let col = col_idx;
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
                >
                    "Import Image"
                </button>
            </div>

            <div class="error-message">
                {move || error_message.get()}
            </div>

            <button type="submit" class="submit-btn">
                "Create Profile"
            </button>
        </form>
    }
}

// Helper function to process imported image
async fn process_image(data: &[u8]) -> Result<PixelArt, String> {
    // Load image from bytes
    let img = image::load_from_memory(data)
        .map_err(|e| format!("Failed to load image: {}", e))?;
    
    // Resize to 32x32
    let resized = img.resize_exact(32, 32, image::imageops::FilterType::Lanczos3);
    
    // Convert to grayscale
    let gray = resized.into_luma8();
    
    // Convert to black and white using threshold
    let threshold = 128u8;
    let mut pixel_art = PixelArt::new();
    
    for (x, y, pixel) in gray.enumerate_pixels() {
        pixel_art.pixels[y as usize][x as usize] = pixel[0] < threshold;
    }
    
    Ok(pixel_art)
} 