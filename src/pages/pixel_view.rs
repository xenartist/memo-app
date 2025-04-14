use leptos::*;
use crate::core::pixel::Pixel;

#[component]
pub fn PixelView(
    #[prop(into)] art: String,
    #[prop(optional)] size: Option<u32>,
    #[prop(optional)] editable: bool,
    #[prop(optional)] on_click: Option<Box<dyn Fn(usize, usize)>>,
) -> impl IntoView {
    let size = size.unwrap_or(64);
    let pixel = create_memo(move |_| {
        Pixel::from_optimal_string(&art).unwrap_or_else(Pixel::new)
    });
    
    let on_click = store_value(on_click);
    
    view! {
        <div class="pixel-grid" style:width=format!("{}px", size) style:height=format!("{}px", size)>
            {move || {
                let current_pixel = pixel.get();
                let (rows, cols) = current_pixel.dimensions();
                
                (0..rows).map(|row| {
                    view! {
                        <div class="pixel-row" style:height=format!("{}%", 100.0 / rows as f32)>
                            {(0..cols).map(|col| {
                                let is_black = current_pixel.get_pixel(row, col);
                                view! {
                                    <div 
                                        class="pixel"
                                        class:black=is_black
                                        class:disabled=(!editable)
                                        style:width=format!("{}%", 100.0 / cols as f32)
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
