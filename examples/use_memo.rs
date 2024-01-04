use eframe::egui;
use egui_hooks::UseHookExt as _;

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        native_options,
        Box::new(|cc| Box::new(MyEguiApp::new(cc))),
    )
    .unwrap();
}

#[derive(Default)]
struct MyEguiApp {}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (count, set_count) = ui.use_state(0usize, ());
            let memo = ui.use_memo(
                || {
                    println!("Calculating memoized value");
                    count.pow(2)
                },
                (count.clone(),),
            );
            ui.label(format!("Count: {}", count));
            ui.label(format!("Memo: {}", memo));
            if ui.button("Increment").clicked() {
                set_count(*count + 1);
            }
        });
    }
}
