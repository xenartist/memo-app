use egui::{CentralPanel, Context, Vec2, FontId, TextStyle};

// Different screens in our application
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    Login,
    NewWallet,
    ImportWallet,
    // We'll add more screens later
}

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
            // Add some space at the top to center the content vertically
            ui.add_space(100.0);
            
            // Center the content horizontally
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

        next_screen
    }
} 