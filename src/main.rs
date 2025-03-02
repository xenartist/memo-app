mod screens;
mod encrypt;

use eframe::{egui, App, CreationContext};
use screens::{
    Screen,
    login::LoginScreen,
    new_wallet::NewWalletScreen,
    import_wallet::ImportWalletScreen,
    main_screen::MainScreen,
};

// Application state
struct MemoApp {
    // Current screen/state of the app
    current_screen: Screen,
    
    // Screen instances
    login_screen: LoginScreen,
    new_wallet_screen: NewWalletScreen,
    import_wallet_screen: ImportWalletScreen,
    main_screen: Option<MainScreen>,
    
    // Seed phrase for wallet
    seed_phrase: Option<String>,
}

impl Default for MemoApp {
    fn default() -> Self {
        Self {
            current_screen: Screen::Login,
            login_screen: LoginScreen::new(),
            new_wallet_screen: NewWalletScreen::new(),
            import_wallet_screen: ImportWalletScreen::new(),
            main_screen: None,
            seed_phrase: None,
        }
    }
}

impl MemoApp {
    // Create a new instance of the app
    fn new(_cc: &CreationContext<'_>) -> Self {
        Self::default()
    }
    
    // Set the seed phrase and create main screen
    fn set_seed_phrase(&mut self, seed_phrase: String) {
        self.seed_phrase = Some(seed_phrase.clone());
        self.main_screen = Some(MainScreen::new(&seed_phrase));
    }
}

impl App for MemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Render the current screen and check if we need to switch to another screen
        let next_screen = match self.current_screen {
            Screen::Login => self.login_screen.render(ctx),
            Screen::NewWallet => {
                let result = self.new_wallet_screen.render(ctx);
                
                // Check if wallet was created successfully
                if let Some(Screen::MainScreen) = result {
                    // Get seed phrase from new wallet screen
                    let seed_phrase = self.new_wallet_screen.get_seed_phrase();
                    self.set_seed_phrase(seed_phrase);
                }
                
                result
            },
            Screen::ImportWallet => {
                let result = self.import_wallet_screen.render(ctx);
                
                // Check if wallet was imported successfully
                if let Some(Screen::MainScreen) = result {
                    // Get seed phrase from import wallet screen
                    let seed_phrase = self.import_wallet_screen.get_seed_phrase();
                    self.set_seed_phrase(seed_phrase);
                }
                
                result
            },
            Screen::MainScreen => {
                if let Some(main_screen) = &mut self.main_screen {
                    main_screen.render(ctx)
                } else {
                    // Fallback to login if main screen is not initialized
                    self.current_screen = Screen::Login;
                    None
                }
            },
        };
        
        // Update the current screen if needed
        if let Some(screen) = next_screen {
            // Reset screens when navigating away from them
            match self.current_screen {
                Screen::NewWallet => {
                    if screen != Screen::NewWallet {
                        // Reset new wallet screen when navigating away
                        self.new_wallet_screen = NewWalletScreen::new();
                    }
                },
                Screen::ImportWallet => {
                    if screen != Screen::ImportWallet {
                        // Reset import wallet screen when navigating away
                        self.import_wallet_screen = ImportWalletScreen::new();
                    }
                },
                _ => {}
            }
            
            // Also reset screens when navigating to them from other screens
            match screen {
                Screen::NewWallet => {
                    if self.current_screen != Screen::NewWallet {
                        // Reset new wallet screen when navigating to it
                        self.new_wallet_screen = NewWalletScreen::new();
                    }
                },
                Screen::ImportWallet => {
                    if self.current_screen != Screen::ImportWallet {
                        // Reset import wallet screen when navigating to it
                        self.import_wallet_screen = ImportWalletScreen::new();
                    }
                },
                _ => {}
            }
            
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
