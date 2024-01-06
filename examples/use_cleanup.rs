use eframe::egui;
use egui_hooks::{hook::state::StateHook, UseHookExt as _};

fn main() {
    eframe::run_native("example", Default::default(), Box::new(|_| Box::new(MyApp))).unwrap();
}

struct MyApp;

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut name = ui.use_state(String::default, ()).into_var();
            let windows = ui.use_state::<Vec<String>>(Default::default, ());
            ui.text_edit_singleline(&mut *name);
            if ui.button("Create a window").clicked() {
                let mut next_windows = windows.as_ref().clone();
                next_windows.push(name.to_string());
                windows.set_next(next_windows);
            }
            for window in windows.as_ref() {
                let mut open = ui
                    .use_hook_as(egui::Id::new(window), StateHook::new(|| true), ())
                    .into_var();
                egui::Window::new(window)
                    .open(&mut open)
                    .show(ui.ctx(), |ui| {
                        let window_cloned = window.clone();
                        ui.use_cleanup(move || println!("Window {} closed", window_cloned), ());
                        ui.label(format!("Hello, {}!", window));
                    });
            }
        });
    }
}
