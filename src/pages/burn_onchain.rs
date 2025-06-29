use leptos::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug)]
pub struct BurnOptions {
    pub personal_collection: bool,  // burn to personal on-chain collection
    pub global_glory_board: bool,   // burn to global glory board
}

impl BurnOptions {
    pub fn new() -> Self {
        Self {
            personal_collection: false,
            global_glory_board: false,
        }
    }
}

#[component]
pub fn BurnOnchain(
    /// control modal visibility
    show_modal: ReadSignal<bool>,
    set_show_modal: WriteSignal<bool>,
    /// transaction signature to burn
    signature: ReadSignal<String>,
    /// callback when user makes a choice
    #[prop(optional)] on_burn_choice: Option<Callback<(String, BurnOptions)>>,
    /// custom close callback (optional)
    #[prop(optional)] on_close: Option<Callback<()>>,
) -> impl IntoView {
    // State for selected options (using checkboxes instead of radio)
    let (personal_collection_checked, set_personal_collection_checked) = create_signal(false);
    let (global_glory_board_checked, set_global_glory_board_checked) = create_signal(false);

    // Handle backdrop click to close modal
    let handle_backdrop_click = move |ev: ev::MouseEvent| {
        if let Some(target) = ev.target() {
            if let Ok(element) = target.dyn_into::<web_sys::HtmlElement>() {
                if element.class_list().contains("burn-onchain-overlay") {
                    set_show_modal.set(false);
                    if let Some(callback) = on_close {
                        callback.call(());
                    }
                }
            }
        }
    };

    // Handle close button click
    let handle_close = move |_| {
        set_show_modal.set(false);
        if let Some(callback) = on_close {
            callback.call(());
        }
    };

    // Handle burn choice confirmation
    let handle_confirm = move |_| {
        let sig = signature.get();
        let burn_options = BurnOptions {
            personal_collection: personal_collection_checked.get(),
            global_glory_board: global_glory_board_checked.get(),
        };
        
        if let Some(callback) = on_burn_choice {
            callback.call((sig, burn_options));
        }
        
        set_show_modal.set(false);
    };

    view! {
        <div 
            class="burn-onchain-overlay"
            class:show=show_modal
            on:click=handle_backdrop_click
        >
            <div class="burn-onchain-modal">
                // Header
                <div class="modal-header">
                    <h3 class="modal-title">
                        <i class="fas fa-fire"></i>
                        " Choose Burn Options"
                    </h3>
                    <button class="close-btn" on:click=handle_close>
                        <i class="fas fa-times"></i>
                    </button>
                </div>

                // Content
                <div class="modal-body">
                    <p class="description">
                        "Select your burn options (you can choose multiple):"
                    </p>

                    <div class="burn-options">
                        // Personal on-chain collection option
                        <label class="burn-option">
                            <input 
                                type="checkbox"
                                checked=personal_collection_checked
                                on:change=move |ev| {
                                    set_personal_collection_checked.set(event_target_checked(&ev));
                                }
                            />
                            <div class="option-content">
                                <div class="option-icon">
                                    <i class="fas fa-archive"></i>
                                </div>
                                <div class="option-text">
                                    <div class="option-title">"Personal On-Chain Collection"</div>
                                    <div class="option-desc">"Add to your personal burn history with detailed records"</div>
                                </div>
                            </div>
                        </label>

                        // Global glory board option
                        <label class="burn-option">
                            <input 
                                type="checkbox"
                                checked=global_glory_board_checked
                                on:change=move |ev| {
                                    set_global_glory_board_checked.set(event_target_checked(&ev));
                                }
                            />
                            <div class="option-content">
                                <div class="option-icon">
                                    <i class="fas fa-trophy"></i>
                                </div>
                                <div class="option-text">
                                    <div class="option-title">"Global Glory Board"</div>
                                    <div class="option-desc">"Compete on the global leaderboard (requires ≥420 MEMO tokens)"</div>
                                </div>
                            </div>
                        </label>
                    </div>

                    // Information note
                    <div class="burn-info">
                        <p class="info-note">
                            <i class="fas fa-info-circle"></i>
                            " Note: You can select both options, one option, or neither."
                        </p>
                        <p class="info-note">
                            <i class="fas fa-trophy"></i>
                            " Global Glory Board requires at least 420 MEMO tokens to participate."
                        </p>
                    </div>
                </div>

                // Footer
                <div class="modal-footer">
                    <button class="btn confirm-btn" on:click=handle_confirm>
                        <i class="fas fa-fire"></i>
                        " Confirm Burn"
                    </button>
                </div>
            </div>
        </div>
    }
} 