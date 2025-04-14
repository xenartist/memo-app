use leptos::*;
use crate::core::pixel::Pixel;

#[component]
pub fn PixelView(
    #[prop(into)] art: String,
    #[prop(optional)] size: Option<u32>,
) -> impl IntoView {
    let size = size.unwrap_or(64);
    
    view! {
        <div class="pixel-grid" style:width=format!("{}px", size) style:height=format!("{}px", size)>
            {
                let pixel = Pixel::from_optimal_string(&art).unwrap_or_else(Pixel::new);
                let (rows, cols) = pixel.dimensions();
                (0..rows).map(|row| {
                    view! {
                        <div class="pixel-row">
                            {(0..cols).map(|col| {
                                let is_black = pixel.get_pixel(row, col);
                                view! {
                                    <div 
                                        class="pixel"
                                        class:black=is_black
                                    />
                                }
                            }).collect_view()}
                        </div>
                    }
                }).collect_view()
            }
        </div>
    }
}
