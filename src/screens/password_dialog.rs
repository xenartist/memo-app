use egui::{Context, Window, TextEdit, RichText, Color32, Vec2};

pub struct PasswordDialog {
    password: String,
    status: String,
    is_open: bool,
    callback: Option<Box<dyn FnOnce(String)>>,
}

impl Default for PasswordDialog {
    fn default() -> Self {
        Self {
            password: String::new(),
            status: String::new(),
            is_open: false,
            callback: None,
        }
    }
}

impl PasswordDialog {
    pub fn new() -> Self {
        Self::default()
    }

    // Request password with a callback
    pub fn request_password<F>(&mut self, callback: F)
    where
        F: FnOnce(String) + 'static,
    {
        self.password = String::new();
        self.status = String::new();
        self.is_open = true;
        self.callback = Some(Box::new(callback));
    }

    // Set error message
    #[allow(dead_code)]
    pub fn set_error(&mut self, error: &str) {
        self.status = error.to_string();
    }

    // Show the password dialog
    pub fn show(&mut self, ctx: &Context) -> bool {
        if !self.is_open {
            return false;
        }

        let mut is_open = self.is_open;
        let mut submitted = false;
        let mut password_clone = self.password.clone();

        // Create a separate variable for the window state
        let mut window_open = is_open;

        Window::new("Enter Password")
            .collapsible(false)
            .resizable(false)
            .default_size(Vec2::new(400.0, 200.0))
            .open(&mut window_open)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label("Please enter your wallet password:");
                    ui.add_space(10.0);
                    
                    // Password input field
                    let password_edit = TextEdit::singleline(&mut password_clone)
                        .password(true)
                        .hint_text("Password")
                        .desired_width(300.0);
                    
                    let response = ui.add(password_edit);
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        submitted = true;
                    }
                    
                    ui.add_space(10.0);
                    
                    // Show status/error message if any
                    if !self.status.is_empty() {
                        ui.label(RichText::new(&self.status).color(Color32::RED));
                        ui.add_space(10.0);
                    }
                    
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            is_open = false;
                        }
                        
                        if ui.button("Submit").clicked() {
                            submitted = true;
                        }
                    });
                });
            });

        // Update the password from the clone
        self.password = password_clone;

        // Update is_open from window_open
        is_open = is_open && window_open;

        // Handle submission
        if submitted && !self.password.is_empty() {
            if let Some(callback) = self.callback.take() {
                callback(self.password.clone());
            }
            self.password.clear();
            is_open = false;
        }

        // Update open state
        self.is_open = is_open;
        
        submitted
    }
} 