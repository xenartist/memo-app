use leptos::*;
use crate::pages::pixel_view::PixelView;

#[component]
pub fn MemoCard(
    image: String,    // image
    signature: String,     // signature
    pubkey: String,       // pubkey
    blocktime: i64,             // blocktime
    amount: f64,                // amount
) -> impl IntoView {
    let pixel_art = "n:3UZcHVQ0*UD`75D)/9W9[@$E#F#+ddL^$7+a/AVJ7R7SKW?0$V@<3DaVT'(V?VHKB=N-%K3bJ^BH-cdGP33]cB9I`&KH*D)X#XF#V$S[VH%CI_=P--_]*T&]^`?>N?.aNJ)V8.W8Z&V/DZ9I+0?0BbD^VV]/0aGa=,G6d456c`#";

    view! {
        <div class="memo-card">
            <div class="pixel-art-container">
                <PixelView
                    art=pixel_art.to_string()
                    size=128
                />
            </div>
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
