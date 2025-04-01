use leptos::*;
use crate::core::session::UserProfile;

#[component]
pub fn ProfilePage() -> impl IntoView {
    view! {
        <div class="profile-page">
            <h2>"Profile Settings"</h2>
            <div class="profile-form">
                <div class="form-group">
                    <label for="username">"Username"</label>
                    <input 
                        type="text"
                        id="username"
                        placeholder="Enter username"
                    />
                </div>
                
                <div class="form-group">
                    <label for="profile-image">"Profile Image"</label>
                    <input 
                        type="text"
                        id="profile-image"
                        placeholder="Enter profile image URL or hex"
                    />
                </div>

                <button class="save-button">
                    "Save Changes"
                </button>
            </div>
        </div>
    }
} 