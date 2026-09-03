use eframe::egui;

struct MyApp;

impl eframe::App for MyApp {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        _frame: &mut eframe::Frame,
    ) {
        let ctx = ui.ctx().clone();
        ui.label("Main Window Test");

        ctx.show_viewport_immediate(
    egui::ViewportId::from_hash_of("test_viewport"),
            egui::ViewportBuilder::default()
                .with_title("TEST")
                .with_inner_size([200.0, 200.0]),
            |_sub_id, _class, | {
                println!("Viewpoert Callback");
            }
        );
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "TEST",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp))),
    )
}