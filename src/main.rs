mod screens;

use eframe::{egui, App, CreationContext};
use screens::{
    Screen,
    login::LoginScreen,
    new_wallet::NewWalletScreen,
    import_wallet::ImportWalletScreen,
};

// Application state
struct MemoApp {
    // Current screen/state of the app
    current_screen: Screen,
    
    // Screen instances
    login_screen: LoginScreen,
    new_wallet_screen: NewWalletScreen,
    import_wallet_screen: ImportWalletScreen,
}

impl Default for MemoApp {
    fn default() -> Self {
        Self {
            current_screen: Screen::Login,
            login_screen: LoginScreen::new(),
            new_wallet_screen: NewWalletScreen::new(),
            import_wallet_screen: ImportWalletScreen::new(),
        }
    }
}

impl MemoApp {
    // Create a new instance of the app
    fn new(_cc: &CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl App for MemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Render the current screen and check if we need to switch to another screen
        let next_screen = match self.current_screen {
            Screen::Login => self.login_screen.render(ctx),
            Screen::NewWallet => self.new_wallet_screen.render(ctx),
            Screen::ImportWallet => self.import_wallet_screen.render(ctx),
        };
        
        // Update the current screen if needed
        if let Some(screen) = next_screen {
            self.current_screen = screen;
        }
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1920.0, 1080.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Memo App",
        native_options,
        Box::new(|cc| Ok(Box::new(MemoApp::new(cc))))
    )
}
