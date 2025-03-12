use egui::{Ui, RichText, Color32};

pub struct SettingsPanel {
    // Add settings fields here
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self {
            // Initialize settings fields
        }
    }
}

impl SettingsPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("Settings");
            ui.add_space(20.0);
            
            // Network settings section
            ui.label(RichText::new("Network Settings").size(18.0).color(Color32::LIGHT_BLUE));
            ui.add_space(10.0);
            
            // RPC URL selection
            ui.label("RPC URL:");
            ui.add_space(5.0);
            
            // X1 Networks row
            ui.horizontal(|ui| {
                ui.label(RichText::new("X1:").color(Color32::DARK_GREEN));
                ui.add_space(5.0);
                if ui.radio_value(&mut true, true, "Testnet").clicked() {
                    // Handle x1 testnet selection
                }
                if ui.radio_value(&mut false, true, "Mainnet").clicked() {
                    // Handle x1 mainnet selection
                }
            });

            ui.add_space(20.0);
            
            // Solana Networks row
            ui.horizontal(|ui| {
                ui.label(RichText::new("Solana:").color(Color32::DARK_RED));
                ui.add_space(5.0);
                if ui.radio_value(&mut false, true, "Devnet").clicked() {
                    // Handle solana devnet selection
                }
                if ui.radio_value(&mut false, true, "Testnet").clicked() {
                    // Handle solana testnet selection
                }
                if ui.radio_value(&mut false, true, "Mainnet").clicked() {
                    // Handle solana mainnet selection
                }
            });
            
            ui.add_space(20.0);
        });
    }
} 