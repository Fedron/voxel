use std::rc::Rc;

use winit::{event::WindowEvent, event_loop::EventLoop};

use crate::{app::Window, generator::WorldGeneratorOptions};

pub struct WorldGeneratorUi {
    window: Rc<Window>,
    egui: egui_glium::EguiGlium,

    seed: String,
    world_size: [String; 3],
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
            world_size: [
                world_generator_options.world_size.x.to_string(),
                world_generator_options.world_size.y.to_string(),
                world_generator_options.world_size.z.to_string(),
            ],
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
                ui.label("Seed:");

                let is_seed_valid = self.seed.parse::<u32>().is_ok();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.seed).text_color(if is_seed_valid {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::RED
                        }),
                    )
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

                ui.label("World Size:");

                ui.horizontal(|ui| {
                    ui.label("X:");
                    let is_world_size_x_valid = self.world_size[0].parse::<u32>().is_ok();
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.world_size[0])
                                .desired_width(20.0)
                                .text_color(if is_world_size_x_valid {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::RED
                                }),
                        )
                        .lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        if let Ok(x) = self.world_size[0].parse() {
                            self.world_generator_options.world_size.x = x;
                        }
                    }

                    ui.label("Y:");
                    let is_world_size_y_valid = self.world_size[1].parse::<u32>().is_ok();
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.world_size[1])
                                .desired_width(20.0)
                                .text_color(if is_world_size_y_valid {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::RED
                                }),
                        )
                        .lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        if let Ok(y) = self.world_size[1].parse() {
                            self.world_generator_options.world_size.y = y;
                        }
                    }

                    ui.label("Z:");
                    let is_world_size_z_valid = self.world_size[2].parse::<u32>().is_ok();
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.world_size[2])
                                .desired_width(20.0)
                                .text_color(if is_world_size_z_valid {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::RED
                                }),
                        )
                        .lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        if let Ok(z) = self.world_size[2].parse() {
                            self.world_generator_options.world_size.z = z;
                        }
                    }
                });

                ui.add(
                    egui::Slider::new(
                        &mut self.world_generator_options.max_terrain_height,
                        0..=self.world_generator_options.world_size.y
                            * self.world_generator_options.chunk_size.y,
                    )
                    .text("Max Terrain Height"),
                );

                let max_dirt_layer_thickness = self.world_generator_options.world_size.y
                    * self.world_generator_options.chunk_size.y
                    - self.world_generator_options.dirt_layer_thickness
                    - 1;
                ui.add(
                    egui::Slider::new(
                        &mut self.world_generator_options.dirt_layer_thickness,
                        0..=max_dirt_layer_thickness,
                    )
                    .text("Dirt Layer Thickness"),
                );

                ui.add(
                    egui::Slider::new(
                        &mut self.world_generator_options.sea_level,
                        0..=self.world_generator_options.world_size.y
                            * self.world_generator_options.chunk_size.y,
                    )
                    .text("Sea Level"),
                );

                ui.add(
                    egui::Slider::new(
                        &mut self.world_generator_options.terrain_smoothness,
                        0.0..=200.0,
                    )
                    .text("Terrain Smoothness"),
                );

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
