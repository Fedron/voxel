use std::rc::Rc;

use winit::{event::WindowEvent, event_loop::EventLoop};

use crate::{app::Window, generator::WorldGeneratorOptions};

pub struct WorldGeneratorUi {
    window: Rc<Window>,
    egui: egui_glium::EguiGlium,

    seed: String,
    pub world_generator_options: WorldGeneratorOptions,
    pub should_generate_world: bool,
}

impl WorldGeneratorUi {
    pub fn new(
        world_generator_options: WorldGeneratorOptions,
        window: Rc<Window>,
        event_loop: &EventLoop<()>,
    ) -> Self {
        Self {
            egui: egui_glium::EguiGlium::new(
                egui::ViewportId::ROOT,
                &window.display,
                &window.winit,
                event_loop,
            ),
            window,

            seed: world_generator_options.seed.to_string(),
            world_generator_options,
            should_generate_world: false,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) {
        let _ = self.egui.on_event(&self.window.winit, event);
    }

    pub fn render(&mut self, frame: &mut glium::Frame) {
        self.egui.run(&self.window.winit, |ctx| {
            egui::Window::new("World Generator").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Seed:");

                    let is_seed_valid = self.seed.parse::<u32>().is_ok();
                    if ui
                        .add(egui::TextEdit::singleline(&mut self.seed).text_color(
                            if is_seed_valid {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::RED
                            },
                        ))
                        .lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        if let Ok(seed) = self.seed.parse() {
                            self.world_generator_options.seed = seed;
                        }
                    }

                    if let Err(_) = self.seed.parse::<u32>() {
                        ui.label("Seed should be a number.");
                    }

                    if ui.button("Random").clicked() {
                        self.seed = rand::random::<u32>().to_string();
                    }
                });

                ui.collapsing("Size Settings", |ui| {
                    ui.label("Chunk Size:");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Slider::new(
                                &mut self.world_generator_options.chunk_size.x,
                                0..=128,
                            )
                            .text("X"),
                        );
                        ui.add(
                            egui::Slider::new(
                                &mut self.world_generator_options.chunk_size.y,
                                0..=128,
                            )
                            .text("Y"),
                        );
                        ui.add(
                            egui::Slider::new(
                                &mut self.world_generator_options.chunk_size.z,
                                0..=128,
                            )
                            .text("Z"),
                        );
                    });
                });

                ui.collapsing("Continent Settings", |ui| {
                    ui.add(
                        egui::Slider::new(
                            &mut self.world_generator_options.continent_frequency,
                            0.0001..=0.1,
                        )
                        .text("Continent Frequency"),
                    );

                    ui.add(
                        egui::Slider::new(
                            &mut self.world_generator_options.continent_lacunarity,
                            1.5..=2.5,
                        )
                        .text("Continent Lacunarity"),
                    );

                    ui.add(
                        egui::Slider::new(&mut self.world_generator_options.sea_level, -1.0..=1.0)
                            .text("Sea Level"),
                    );
                });

                ui.separator();

                if ui
                    .add(egui::Button::new("Generate"))
                    .on_hover_ui(|ui| {
                        ui.label("Generate a new world with the given seed.");
                    })
                    .clicked()
                {
                    self.should_generate_world = true;
                    self.world_generator_options.seed = self.seed.parse().expect("to parse seed");
                }
            });
        });

        self.egui.paint(&self.window.display, frame);
    }
}
