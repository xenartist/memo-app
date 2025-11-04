use leptos::*;
use leptos::html::Canvas;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d, MouseEvent};
use wasm_bindgen::{JsCast, JsValue};
use crate::core::pixel::Pixel;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn PixelView(
    #[prop(into)] art: String,
    #[prop(optional)] size: Option<u32>,
    #[prop(optional)] editable: bool,
    #[prop(optional)] on_click: Option<Box<dyn Fn(usize, usize)>>,
    #[prop(optional)] show_grid: Option<bool>,
) -> impl IntoView {
    let display_size = size.unwrap_or(64);
    let show_grid = show_grid.unwrap_or(true);
    
    // create memo for pixel data
    let pixel_data = create_memo(move |_| {
        Pixel::from_optimal_string(&art).unwrap_or_else(Pixel::new)
    });
    
    // Canvas element reference
    let canvas_ref = create_node_ref::<Canvas>();
    
    // store click callback
    let on_click = store_value(on_click);
    
    // get Canvas element helper function
    let get_canvas = move || -> Option<HtmlCanvasElement> {
        canvas_ref
            .get()
            .and_then(|element| {
                element
                    .unchecked_ref::<web_sys::Element>()
                    .dyn_ref::<HtmlCanvasElement>()
                    .cloned()
            })
    };
    
    // draw Canvas function
    let draw_canvas = move || {
        if let Some(canvas) = get_canvas() {
            // get 2d rendering context
            let context = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<CanvasRenderingContext2d>()
                .unwrap();
            
            let pixel = pixel_data.get();
            let (rows, cols) = pixel.dimensions();
            let canvas_size = display_size as f64;
            let pixel_size = canvas_size / rows as f64;
            
            // clear Canvas
            context.clear_rect(0.0, 0.0, canvas_size, canvas_size);
            
            // draw white background
            context.set_fill_style(&JsValue::from_str("white"));
            context.fill_rect(0.0, 0.0, canvas_size, canvas_size);
            
            // draw black pixels
            context.set_fill_style(&JsValue::from_str("black"));
            for row in 0..rows {
                for col in 0..cols {
                    if pixel.get_pixel(row, col) {
                        let x = col as f64 * pixel_size;
                        let y = row as f64 * pixel_size;
                        context.fill_rect(x, y, pixel_size, pixel_size);
                    }
                }
            }
            
            // draw grid lines (if enabled and editable)
            if show_grid && editable {
                context.set_stroke_style(&JsValue::from_str("#ddd"));
                context.set_line_width(0.5);
                
                // vertical lines
                for i in 0..=cols {
                    let x = i as f64 * pixel_size;
                    context.begin_path();
                    context.move_to(x, 0.0);
                    context.line_to(x, canvas_size);
                    context.stroke();
                }
                
                // horizontal lines
                for i in 0..=rows {
                    let y = i as f64 * pixel_size;
                    context.begin_path();
                    context.move_to(0.0, y);
                    context.line_to(canvas_size, y);
                    context.stroke();
                }
            }
        }
    };
    
    // respond to data changes and automatically redraw
    create_effect(move |_| {
        pixel_data.track();
        // use request_animation_frame to ensure DOM is updated
        request_animation_frame(move || {
            draw_canvas();
        });
    });
    
    // handle mouse click event
    let handle_canvas_click = move |event: MouseEvent| {
        if !editable {
            return;
        }
        
        if let Some(canvas) = get_canvas() {
            let rect = canvas.get_bounding_client_rect();
            let pixel = pixel_data.get();
            let (rows, cols) = pixel.dimensions();
            
            // calculate coordinates relative to Canvas
            let x = event.client_x() as f64 - rect.left();
            let y = event.client_y() as f64 - rect.top();
            
            // convert to pixel coordinates
            let pixel_size = display_size as f64 / rows as f64;
            let pixel_col = (x / pixel_size) as usize;
            let pixel_row = (y / pixel_size) as usize;
            
            // ensure coordinates are within valid range
            if pixel_row < rows && pixel_col < cols {
                on_click.with_value(|f| {
                    if let Some(handler) = f.as_ref() {
                        handler(pixel_row, pixel_col);
                    }
                });
            }
        }
    };
    
    view! {
        <canvas
            node_ref=canvas_ref
            width=display_size
            height=display_size
            class="pixel-grid"
            class:editable=editable
            class:disabled=(!editable)
            style=format!(
                "width: {}px; height: {}px; display: block;",
                display_size, display_size
            )
            on:click=handle_canvas_click
        />
    }
}

// request_animation_frame helper function
fn request_animation_frame(f: impl FnOnce() + 'static) {
    use wasm_bindgen::prelude::*;
    
    let mut f = Some(f);
    let closure = Closure::wrap(Box::new(move || {
        if let Some(f) = f.take() {
            f();
        }
    }) as Box<dyn FnMut()>);
    
    web_sys::window()
        .unwrap()
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .unwrap();
    
    closure.forget();
}

// lazy loading pixel view
#[component]
pub fn LazyPixelView(
    art: String,
    size: u32,
) -> impl IntoView {
    let (is_loaded, set_is_loaded) = create_signal(false);
    
    // use signal to store art string, avoid moving issues
    let (art_signal, _) = create_signal(art);
    
    // async decode, add delay to avoid blocking UI
    create_effect(move |_| {
        spawn_local(async move {
            // Canvas rendering is fast, can shorten delay
            TimeoutFuture::new(50).await;
            set_is_loaded.set(true);
        });
    });
    
    view! {
        {move || {
            if is_loaded.get() {
                view! {
                    <PixelView
                        art={art_signal.get()}
                        size=size
                        editable=false
                        show_grid=false
                    />
                }.into_view()
            } else {
                view! {
                    <div class="pixel-loading" style="display: flex; align-items: center; justify-content: center; height: 128px; color: #666; background-color: #f8f9fa; border-radius: 6px;">
                        <i class="fas fa-spinner fa-spin" style="margin-right: 8px;"></i>
                        <span>"Loading..."</span>
                    </div>
                }.into_view()
            }
        }}
    }
} 