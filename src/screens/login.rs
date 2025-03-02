use egui::{CentralPanel, Context, Vec2, FontId, TextStyle, Frame};
use super::Screen;

pub struct LoginScreen {
    // Add any login screen specific state here
}

impl Default for LoginScreen {
    fn default() -> Self {
        Self {}
    }
}

impl LoginScreen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(&mut self, ctx: &Context) -> Option<Screen> {
        let mut next_screen = None;

        // Set font style
        let mut style = (*ctx.style()).clone();
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(20.0, egui::FontFamily::Proportional)
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(30.0, egui::FontFamily::Proportional)
        );
        ctx.set_style(style);

        CentralPanel::default().show(ctx, |ui| {
            // Get available size
            let available_size = ui.available_size();
            
            // Calculate vertical position to center the form
            let form_height = 250.0; // Approximate height of our form
            let vertical_margin = (available_size.y - form_height) / 2.0;
            if vertical_margin > 0.0 {
                ui.add_space(vertical_margin);
            }
            
            // Center content horizontally
            ui.vertical_centered(|ui| {
                // Set a fixed width for the form
                let form_width = 400.0;
                
                // Create a frame for the form with fixed width
                Frame::group(ui.style())
                    .inner_margin(30.0)
                    .outer_margin(10.0)
                    .show(ui, |ui| {
                        // Set the maximum width for the content
                        ui.set_max_width(form_width);
                        
                        ui.vertical_centered(|ui| {
                            ui.heading("Welcome to Memo World");
                            ui.add_space(30.0);
                            
                            // Make buttons a bit larger
                            let button_size = Vec2::new(250.0, 60.0);
                            
                            if ui.add_sized(button_size, egui::Button::new("New Wallet")).clicked() {
                                next_screen = Some(Screen::NewWallet);
                            }
                            
                            ui.add_space(20.0);
                            
                            if ui.add_sized(button_size, egui::Button::new("Import Wallet")).clicked() {
                                next_screen = Some(Screen::ImportWallet);
                            }
                        });
                    });
            });
        });

        next_screen
    }
} 