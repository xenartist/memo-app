use egui::{CentralPanel, Context, FontId, TextStyle};
use super::Screen;

pub struct NewWalletScreen {
    // Add any new wallet screen specific state here
}

impl Default for NewWalletScreen {
    fn default() -> Self {
        Self {}
    }
}

impl NewWalletScreen {
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
            ui.heading("Create New Wallet");
            ui.add_space(20.0);
            // We'll implement this later
            
            if ui.button("Back").clicked() {
                next_screen = Some(Screen::Login);
            }
        });

        next_screen
    }
} 