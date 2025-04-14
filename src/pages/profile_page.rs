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
use solana_sdk::signature::Keypair;
use hex;
use crate::core::wallet::{derive_keypair_from_seed, get_default_derivation_path};
use gloo_timers;
use crate::pages::pixel_view::PixelView;

#[derive(Clone, Copy, PartialEq)]
enum ProfileFormState {
    Create,           // create new profile
    View,            // view existing profile (not editable)
    Edit,            // edit existing profile (editable)
}

#[component]
pub fn ProfilePage(
    session: RwSignal<Session>
) -> impl IntoView {
    let (user_profile, set_user_profile) = create_signal::<Option<UserProfile>>(None);
    let (is_loading, set_is_loading) = create_signal(true);
    let form_state = create_rw_signal(ProfileFormState::Create);

    // get cached profile from session (if there is any)
    if let Some(profile) = session.get().get_user_profile() {
        set_user_profile.set(Some(profile));
        form_state.set(ProfileFormState::View);
        set_is_loading.set(false);
    }

    // fetch profile from RPC
    create_effect(move |_| {
        // if there is already data, no need to request
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
                        log::info!("Raw profile result: {}", result);
                        match parse_user_profile(&result) {
                            Ok(profile) => {
                                log::info!("Successfully parsed profile: {:?}", profile);
                                current_session.set_user_profile(Some(profile.clone()));
                                session.set(current_session);
                                set_user_profile.set(Some(profile));
                                form_state.set(ProfileFormState::View);
                            }
                            Err(e) => {
                                log::error!("Failed to parse profile: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to get user profile: {:?}", e);
                    }
                }
            }
            set_is_loading.set(false);  // set loading state to false after request
        });
    });

    view! {
        <div class="profile-page">
            <h2>"User Profile"</h2>
            
            <div class="profile-content">
                {move || {
                    if is_loading.get() {
                        view! { 
                            <div class="profile-content-inner">
                                <div class="loading">"Loading..."</div>
                            </div>
                        }
                    } else {
                        view! {
                            <div class="profile-content-inner">
                                <ProfileForm 
                                    session=session
                                    existing_profile=user_profile.get()
                                    form_state=form_state
                                />
                            </div>
                        }
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn ProfileForm(
    session: RwSignal<Session>,
    existing_profile: Option<UserProfile>,
    form_state: RwSignal<ProfileFormState>,
) -> impl IntoView {
    let (username, set_username) = create_signal(
        existing_profile.as_ref().map(|p| p.username.clone()).unwrap_or_default()
    );
    
    let (pixel_art, set_pixel_art) = create_signal(
        existing_profile.as_ref()
            .and_then(|p| Pixel::from_optimal_string(&p.profile_image))
            .unwrap_or_else(Pixel::new)
    );

    let (error_message, set_error_message) = create_signal(String::new());
    let (is_submitting, set_is_submitting) = create_signal(false);
    let (show_confirm_dialog, set_show_confirm_dialog) = create_signal(false);

    // Handle pixel click
    let handle_pixel_click = move |row: usize, col: usize| {
        // only allow editing in Create or Edit state
        if matches!(form_state.get(), ProfileFormState::Create | ProfileFormState::Edit) {
            let mut new_art = pixel_art.get();
            new_art.toggle_pixel(row, col);
            set_pixel_art.set(new_art);
        }
    };

    // Handle image import
    let handle_import = move |ev: MouseEvent| {
        // only allow import in Create or Edit state
        if !matches!(form_state.get(), ProfileFormState::Create | ProfileFormState::Edit) {
            return;
        }

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
                
                let onload = Closure::wrap(Box::new(move |_: ProgressEvent| {
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

    // update profile
    let handle_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        
        // process different form states
        match form_state.get() {
            ProfileFormState::Create => {
                let current_username = username.get();
                let current_pixel_art = pixel_art.get();

                // validate username cannot be empty
                if current_username.trim().is_empty() {
                    set_error_message.set("Username cannot be empty".to_string());
                    return;
                }

                set_is_submitting.set(true);
                set_error_message.set(String::new());

                let session_clone = session;
                let set_error_clone = set_error_message;
                let set_submitting_clone = set_is_submitting;
                let form_state_clone = form_state;

                spawn_local(async move {
                    let mut current_session = session_clone.get();
                    
                    match current_session.get_public_key() {
                        Ok(pubkey) => {
                            match current_session.get_seed() {
                                Ok(seed) => {
                                    match hex::decode(&seed) {
                                        Ok(seed_bytes) => {
                                            match seed_bytes.try_into() as Result<[u8; 64], Vec<u8>> {
                                                Ok(seed_array) => {
                                                    match derive_keypair_from_seed(
                                                        &seed_array,
                                                        get_default_derivation_path()
                                                    ) {
                                                        Ok((keypair, _)) => {
                                                            let rpc = RpcConnection::new();
                                                            
                                                            match rpc.initialize_user_profile(
                                                                &pubkey,
                                                                &current_username,
                                                                &current_pixel_art.to_optimal_string(),
                                                                &keypair.to_bytes()
                                                            ).await {
                                                                Ok(_) => {
                                                                    // create new user profile object
                                                                    let new_profile = UserProfile {
                                                                        pubkey: pubkey.clone(),
                                                                        username: current_username,
                                                                        total_minted: 0,
                                                                        total_burned: 0,
                                                                        mint_count: 0,
                                                                        burn_count: 0,
                                                                        profile_image: current_pixel_art.to_optimal_string(),
                                                                        created_at: js_sys::Date::now() as i64,
                                                                        last_updated: js_sys::Date::now() as i64,
                                                                    };

                                                                    // update user profile in session
                                                                    current_session.set_user_profile(Some(new_profile));
                                                                    session_clone.set(current_session);

                                                                    // update form state
                                                                    form_state_clone.set(ProfileFormState::View);
                                                                    set_error_clone.set("Profile created successfully!".to_string());
                                                                }
                                                                Err(e) => {
                                                                    set_error_clone.set(format!("Failed to create profile: {}", e));
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_error_clone.set(format!("Failed to derive keypair: {:?}", e));
                                                        }
                                                    }
                                                }
                                                Err(original_vec) => {
                                                    set_error_clone.set(format!("Invalid seed length: {} (expected 64)", original_vec.len()));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            set_error_clone.set(format!("Failed to decode seed: {}", e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    set_error_clone.set(format!("Failed to get seed: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            set_error_clone.set(format!("Failed to get public key: {}", e));
                        }
                    }

                    set_submitting_clone.set(false);
                });
            }
            ProfileFormState::Edit => {
                let current_username = username.get();
                let current_pixel_art = pixel_art.get();

                set_is_submitting.set(true);
                set_error_message.set(String::new());

                let session_clone = session;
                let set_error_clone = set_error_message;
                let set_submitting_clone = set_is_submitting;
                let form_state_clone = form_state;
                let existing_profile_clone = existing_profile.clone();
                let username_for_update = current_username.clone();

                spawn_local(async move {
                    let mut current_session = session_clone.get();
                    
                    match current_session.get_public_key() {
                        Ok(pubkey) => {
                            match current_session.get_seed() {
                                Ok(seed) => {
                                    match hex::decode(&seed) {
                                        Ok(seed_bytes) => {
                                            match seed_bytes.try_into() as Result<[u8; 64], Vec<u8>> {
                                                Ok(seed_array) => {
                                                    match derive_keypair_from_seed(
                                                        &seed_array,
                                                        get_default_derivation_path()
                                                    ) {
                                                        Ok((keypair, _)) => {
                                                            let rpc = RpcConnection::new();
                                                            
                                                            match rpc.update_user_profile(
                                                                &pubkey,
                                                                Some(username_for_update),
                                                                Some(current_pixel_art.to_optimal_string()),
                                                                &keypair.to_bytes()
                                                            ).await {
                                                                Ok(_) => {
                                                                    if let Some(mut updated_profile) = existing_profile_clone {
                                                                        updated_profile.username = current_username;
                                                                        updated_profile.profile_image = current_pixel_art.to_optimal_string();

                                                                        current_session.set_user_profile(Some(updated_profile));
                                                                        session_clone.set(current_session);

                                                                        set_error_clone.set("Profile updated successfully!".to_string());
                                                                        form_state_clone.set(ProfileFormState::View);
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    set_error_clone.set(format!("Failed to update profile: {}", e));
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_error_clone.set(format!("Failed to derive keypair: {:?}", e));
                                                        }
                                                    }
                                                }
                                                Err(original_vec) => {
                                                    set_error_clone.set(format!("Invalid seed length: {} (expected 64)", original_vec.len()));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            set_error_clone.set(format!("Failed to decode seed: {}", e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    set_error_clone.set(format!("Failed to get seed: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            set_error_clone.set(format!("Failed to get public key: {}", e));
                        }
                    }

                    set_submitting_clone.set(false);
                });
            }
            ProfileFormState::View => {
                // View state does not handle submission
                return;
            }
        }
    };

    // delete profile
    let handle_delete = move |ev: MouseEvent| {
        ev.prevent_default();
        
        if !matches!(form_state.get(), ProfileFormState::View) {
            return;
        }

        // show confirm dialog
        set_show_confirm_dialog.set(true);
    };

    // confirm delete
    let handle_confirm_delete = move || {
        set_show_confirm_dialog.set(false);
        set_is_submitting.set(true);
        set_error_message.set(String::new());

        let session_clone = session;
        let set_error_clone = set_error_message;
        let set_submitting_clone = set_is_submitting;
        let form_state_clone = form_state;
        let set_username_clone = set_username;
        let set_pixel_art_clone = set_pixel_art;

        spawn_local(async move {
            let mut current_session = session_clone.get();
            
            match current_session.get_public_key() {
                Ok(pubkey) => {
                    match current_session.get_seed() {
                        Ok(seed) => {
                            match hex::decode(&seed) {
                                Ok(seed_bytes) => {
                                    match seed_bytes.try_into() as Result<[u8; 64], Vec<u8>> {
                                        Ok(seed_array) => {
                                            match derive_keypair_from_seed(
                                                &seed_array,
                                                get_default_derivation_path()
                                            ) {
                                                Ok((keypair, _)) => {
                                                    let rpc = RpcConnection::new();
                                                    
                                                    match rpc.close_user_profile(
                                                        &pubkey,
                                                        &keypair.to_bytes()
                                                    ).await {
                                                        Ok(_) => {
                                                            // clear user profile in session
                                                            current_session.set_user_profile(None);
                                                            session_clone.set(current_session);

                                                            // reset form state
                                                            form_state_clone.set(ProfileFormState::Create);
                                                            // clear username
                                                            set_username_clone.set(String::new());
                                                            // reset pixel art to new blank canvas
                                                            set_pixel_art_clone.set(Pixel::new());
                                                            
                                                            set_error_clone.set("Profile deleted successfully!".to_string());
                                                        }
                                                        Err(e) => {
                                                            set_error_clone.set(format!("Failed to delete profile: {}", e));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    set_error_clone.set(format!("Failed to derive keypair: {:?}", e));
                                                }
                                            }
                                        }
                                        Err(original_vec) => {
                                            set_error_clone.set(format!("Invalid seed length: {} (expected 64)", original_vec.len()));
                                        }
                                    }
                                }
                                Err(e) => {
                                    set_error_clone.set(format!("Failed to decode seed: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            set_error_clone.set(format!("Failed to get seed: {}", e));
                        }
                    }
                }
                Err(e) => {
                    set_error_clone.set(format!("Failed to get public key: {}", e));
                }
            }

            set_submitting_clone.set(false);
        });
    };

    // cancel delete
    let handle_cancel_delete = move || {
        set_show_confirm_dialog.set(false);
    };

    view! {
        <form class="profile-form" on:submit=handle_submit>
            <h3>
                {move || match form_state.get() {
                    ProfileFormState::Create => "Create Your Profile",
                    ProfileFormState::View => "Your Profile",
                    ProfileFormState::Edit => "Edit Your Profile",
                }}
            </h3>
            
            <div class="form-group">
                <label for="username">"Username"</label>
                <input 
                    type="text"
                    id="username"
                    maxlength="32"
                    placeholder="Enter your username (max 32 characters)"
                    autocomplete="off"
                    on:input=move |ev| {
                        let input = event_target::<HtmlInputElement>(&ev);
                        set_username.set(input.value());
                    }
                    prop:value=username
                    prop:disabled=move || {
                        is_submitting.get() || !matches!(form_state.get(), ProfileFormState::Create | ProfileFormState::Edit)
                    }
                />
            </div>

            <div class="pixel-art-editor">
                <div class="pixel-art-header">
                    <label>"Profile Image (32x32 Pixel Art)"</label>
                    <button 
                        type="button"
                        class="import-btn"
                        class:hidden=move || !matches!(form_state.get(), ProfileFormState::Create | ProfileFormState::Edit)
                        on:click=handle_import
                        prop:disabled=is_submitting
                    >
                        "Import Image"
                    </button>
                </div>
                {move || {
                    let art_string = pixel_art.get().to_optimal_string();
                    let is_editable = matches!(form_state.get(), ProfileFormState::Create | ProfileFormState::Edit);
                    let click_handler = Box::new(move |row, col| {
                        let mut new_art = pixel_art.get();
                        new_art.toggle_pixel(row, col);
                        set_pixel_art.set(new_art);
                    });
                    
                    view! {
                        <PixelView
                            art=art_string
                            size=256
                            grid_size=32
                            editable=is_editable
                            on_click=click_handler
                        />
                    }
                }}
            </div>

            {move || {
                let message = error_message.get();
                view! {
                    <div class="error-message" 
                        class:success=message.contains("success")
                        style:display={if message.is_empty() { "none" } else { "block" }}
                    >
                        {message}
                    </div>
                }
            }}

            <div class="button-group">
                {move || match form_state.get() {
                    ProfileFormState::Create => view! {
                        <div class="button-group create-mode">
                            <button 
                                type="submit" 
                                class="create-btn"
                                prop:disabled=is_submitting
                            >
                                {move || if is_submitting.get() { "Creating Profile..." } else { "Create Profile" }}
                            </button>
                        </div>
                    },
                    ProfileFormState::View => view! {
                        <div class="button-group view-mode">
                            <button 
                                type="button" 
                                class="edit-btn"
                                on:click=move |_| form_state.set(ProfileFormState::Edit)
                                prop:disabled=is_submitting
                            >
                                "Edit Profile"
                            </button>
                            <button 
                                type="button"
                                class="delete-btn"
                                on:click=handle_delete
                                prop:disabled=is_submitting
                            >
                                {move || if is_submitting.get() { "Deleting Profile..." } else { "Delete Profile" }}
                            </button>
                        </div>
                    },
                    ProfileFormState::Edit => view! {
                        <div class="button-group edit-mode">
                            <button 
                                type="submit" 
                                class="update-btn"
                                prop:disabled=is_submitting
                            >
                                {move || if is_submitting.get() { "Updating Profile..." } else { "Update Profile" }}
                            </button>
                            <button 
                                type="button"
                                class="cancel-btn"
                                on:click=move |_| form_state.set(ProfileFormState::View)
                                prop:disabled=is_submitting
                            >
                                "Cancel"
                            </button>
                        </div>
                    }
                }}
            </div>

            // confirm dialog
            {move || if show_confirm_dialog.get() {
                view! {
                    <div class="confirm-modal-overlay">
                        <div class="confirm-modal-content">
                            <div class="confirm-modal-header">
                                <h4>"Delete Profile"</h4>
                            </div>
                            <div class="confirm-modal-body">
                                <p>"Are you sure you want to delete your profile? This action cannot be undone and you will need to create a new profile."</p>
                            </div>
                            <div class="confirm-modal-footer">
                                <button
                                    type="button"
                                    class="confirm-modal-btn cancel"
                                    on:click=move |_| handle_cancel_delete()
                                >
                                    "Cancel"
                                </button>
                                <button
                                    type="button"
                                    class="confirm-modal-btn delete"
                                    on:click=move |_| handle_confirm_delete()
                                >
                                    "Delete"
                                </button>
                            </div>
                        </div>
                    </div>
                }
            } else {
                view! { <div></div> }
            }}
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