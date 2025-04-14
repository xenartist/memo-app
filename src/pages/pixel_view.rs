use leptos::*;
use crate::core::pixel::Pixel;

#[component]
pub fn PixelView(
    #[prop(into)] art: String,
    #[prop(optional)] size: Option<u32>,      // display size
    #[prop(optional)] grid_size: Option<u32>, // grid size (32x32 or 64x64)
    #[prop(optional)] editable: bool,
    #[prop(optional)] on_click: Option<Box<dyn Fn(usize, usize)>>,
) -> impl IntoView {
    let display_size = size.unwrap_or(64);
    let grid_size = grid_size.unwrap_or(32);
    
    let pixel = create_memo(move |_| {
        Pixel::from_optimal_string(&art).unwrap_or_else(Pixel::new)
    });
    
    let on_click = store_value(on_click);
    
    view! {
        <div 
            class="pixel-grid"
            class:grid-32=move || grid_size == 32
            class:grid-64=move || grid_size == 64
            style:width=format!("{}px", display_size)
            style:height=format!("{}px", display_size)
        >
            {move || {
                let current_pixel = pixel.get();
                let (rows, cols) = current_pixel.dimensions();
                
                (0..rows).map(|row| {
                    view! {
                        <div class="pixel-row">
                            {(0..cols).map(|col| {
                                let is_black = current_pixel.get_pixel(row, col);
                                view! {
                                    <div 
                                        class="pixel"
                                        class:black=is_black
                                        class:disabled=(!editable)
                                        on:click=move |_| {
                                            if editable {
                                                on_click.with_value(|f| {
                                                    if let Some(handler) = f.as_ref() {
                                                        handler(row, col);
                                                    }
                                                });
                                            }
                                        }
                                    />
                                }
                            }).collect_view()}
                        </div>
                    }
                }).collect_view()
            }}
        </div>
    }
}
