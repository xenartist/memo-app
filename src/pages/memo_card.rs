use leptos::*;

#[component]
pub fn MemoCard(
    image: String,    // image
    signature: String,     // signature
    pubkey: String,       // pubkey
    blocktime: i64,             // blocktime
    amount: f64,                // amount
) -> impl IntoView {
    view! {
        <div class="memo-card">
            // image container, fixed 64x64 size
            <div class="image-container">
                <img 
                    src={image}
                    alt="Image"
                    width="64"
                    height="64"
                />
            </div>
            
            // memo info
            <div class="memo-info">
                <div class="signature">
                    "Signature: " {signature}
                </div>
                <div class="pubkey">
                    "Pubkey: " {pubkey}
                </div>
                <div class="blocktime">
                    "Block Time: " {blocktime}
                </div>
                <div class="amount">
                    "Amount: " {amount}
                </div>
            </div>
        </div>
    }
}
