// Export screen modules
pub mod login;
pub mod new_wallet;
pub mod import_wallet;
pub mod main_screen;
pub mod panels;
pub mod password_dialog;

// Different screens in our application
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    Login,
    NewWallet,
    ImportWallet,
    MainScreen,
} 