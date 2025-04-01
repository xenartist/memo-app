use leptos::*;
use log;
use web_sys::{HtmlInputElement, MouseEvent};
use crate::core::session::{Session, UserProfile, parse_user_profile};
use crate::core::rpc::RpcConnection;

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
    
    // handle pixel click
    let handle_pixel_click = move |row: usize, col: usize| {
        let mut new_art = pixel_art.get();
        new_art.pixels[row][col] = !new_art.pixels[row][col];
        set_pixel_art.set(new_art);
    };

    // handle import image
    let handle_import = move |ev: MouseEvent| {
        ev.prevent_default();
        // TODO: implement image import logic
    };

    view! {
        <div class="create-profile-form">
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
                    class="import-btn"
                    on:click=handle_import
                >
                    "Import Image"
                </button>
            </div>

            <button class="submit-btn">
                "Create Profile"
            </button>
        </div>
    }
} 