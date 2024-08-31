use eframe::egui;
use egui_hooks::UseHookExt as _;

fn main() {
    eframe::run_native(
        "example",
        Default::default(),
        Box::new(|_| Ok(Box::new(MyApp))),
    )
    .unwrap();
}

struct MyApp;

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let count = ui.use_state(|| 0usize, ());
            ui.label(format!("Count: {}", count));
            ui.label(format!("Previous count: {:?}", count.previous()));
            if ui.button("Increment").clicked() {
                count.set_next(*count + 1);
            }
        });
    }
}
