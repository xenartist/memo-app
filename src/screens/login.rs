use egui::{CentralPanel, Context, Vec2, FontId, TextStyle, Frame, TextEdit, RichText};
use super::Screen;
use crate::core::wallet::Wallet;

pub struct LoginScreen {
    // Password for unlocking wallet
    password: String,
    // Status message (for error display)
    status_message: String,
    // Flag to indicate if wallet file exists
    wallet_exists: bool,
    // Wallet address (after successful login)
    wallet_address: Option<String>,
}

impl Default for LoginScreen {
    fn default() -> Self {
        // Check if wallet file exists
        let wallet_exists = Wallet::wallet_file_exists();
        
        Self {
            password: String::new(),
            status_message: String::new(),
            wallet_exists,
            wallet_address: None,
        }
    }
}

impl LoginScreen {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Decrypt wallet file and get wallet address
    pub fn decrypt_wallet(&mut self) -> Result<(), String> {
        // Use the Wallet module to load the wallet
        let wallet = Wallet::load_wallet(&self.password)?;
        
        // Store the wallet address
        self.wallet_address = Some(wallet.address.clone());
        
        Ok(())
    }

    pub fn render(&mut self, ctx: &Context) -> Option<Screen> {
        let mut next_screen = None;

        // Set font style
        let mut style = (*ctx.style()).clone();
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(22.0, egui::FontFamily::Proportional)
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(32.0, egui::FontFamily::Proportional)
        );
        style.text_styles.insert(
            TextStyle::Body,
            FontId::new(22.0, egui::FontFamily::Proportional)
        );
        ctx.set_style(style);

        CentralPanel::default().show(ctx, |ui| {
            // Get available size
            let available_size = ui.available_size();
            
            // Calculate vertical position to center the form
            let form_height = if self.wallet_exists { 250.0 } else { 300.0 }; // Approximate height of our form
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
                            
                            if self.wallet_exists {
                                // Wallet exists, show unlock form
                                ui.label(RichText::new("Enter password to unlock your wallet:").size(22.0));
                                ui.add_space(10.0);
                                
                                // Password field
                                ui.add(TextEdit::singleline(&mut self.password)
                                    .password(true)
                                    .hint_text("Enter password")
                                    .desired_width(300.0)
                                    .font(FontId::new(22.0, egui::FontFamily::Proportional)));
                                
                                ui.add_space(20.0);
                                
                                // Make button a bit larger
                                let button_size = Vec2::new(250.0, 50.0);
                                
                                if ui.add_sized(button_size, egui::Button::new("Unlock Wallet")).clicked() {
                                    // Try to decrypt wallet
                                    match self.decrypt_wallet() {
                                        Ok(_) => {
                                            // Return with MainScreen
                                            return next_screen = Some(Screen::MainScreen);
                                        }
                                        Err(e) => {
                                            self.status_message = format!("Error: {}", e);
                                        }
                                    }
                                }
                                
                                // Display status message if any
                                if !self.status_message.is_empty() {
                                    ui.add_space(10.0);
                                    ui.colored_label(
                                        egui::Color32::RED,
                                        &self.status_message
                                    );
                                }
                            } else {
                                // No wallet exists, show options to create or import
                                // Make buttons a bit larger
                                let button_size = Vec2::new(250.0, 60.0);
                                
                                if ui.add_sized(button_size, egui::Button::new("New Wallet")).clicked() {
                                    next_screen = Some(Screen::NewWallet);
                                }
                                
                                ui.add_space(20.0);
                                
                                if ui.add_sized(button_size, egui::Button::new("Import Wallet")).clicked() {
                                    next_screen = Some(Screen::ImportWallet);
                                }
                            }
                        });
                    });
            });
        });

        next_screen
    }
    
    // Get the wallet address
    pub fn get_wallet_address(&self) -> Option<String> {
        self.wallet_address.clone()
    }
} 