use eframe::{egui, App, CreationContext};
use egui::{CentralPanel, Vec2};

// Application state
struct MemoApp {
    // Track the current screen/state of the app
    current_screen: Screen,
}

// Different screens in our application
enum Screen {
    Login,
    NewWallet,
    ImportWallet,
    // We'll add more screens later
}

impl Default for MemoApp {
    fn default() -> Self {
        Self {
            current_screen: Screen::Login,
        }
    }
}

impl MemoApp {
    // Create a new instance of the app
    fn new(_cc: &CreationContext<'_>) -> Self {
        Self::default()
    }
    
    // Render the login screen with two buttons
    fn render_login_screen(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            // Add some space at the top to center the content vertically
            ui.add_space(100.0);
            
            // Center the content horizontally
            ui.vertical_centered(|ui| {
                ui.heading("Welcome to Memo App");
                ui.add_space(20.0);
                
                // Make buttons a bit larger
                let button_size = Vec2::new(200.0, 50.0);
                
                if ui.add_sized(button_size, egui::Button::new("New Wallet")).clicked() {
                    self.current_screen = Screen::NewWallet;
                }
                
                ui.add_space(10.0);
                
                if ui.add_sized(button_size, egui::Button::new("Import Wallet")).clicked() {
                    self.current_screen = Screen::ImportWallet;
                }
            });
        });
    }

    // Placeholder for the new wallet screen
    fn render_new_wallet_screen(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Create New Wallet");
            // We'll implement this later
            
            if ui.button("Back").clicked() {
                self.current_screen = Screen::Login;
            }
        });
    }

    // Placeholder for the import wallet screen
    fn render_import_wallet_screen(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Import Existing Wallet");
            // We'll implement this later
            
            if ui.button("Back").clicked() {
                self.current_screen = Screen::Login;
            }
        });
    }
}

impl App for MemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.current_screen {
            Screen::Login => self.render_login_screen(ctx),
            Screen::NewWallet => self.render_new_wallet_screen(ctx),
            Screen::ImportWallet => self.render_import_wallet_screen(ctx),
        }
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Memo App",
        native_options,
        Box::new(|cc| Box::new(MemoApp::new(cc)))
    )
}
