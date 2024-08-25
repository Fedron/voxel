#[macro_use]
extern crate glium;
use std::rc::Rc;

use app::{App, AppBehaviour, Window};
use camera::{Camera, CameraController, Projection};
use chunk::CHUNK_SIZE;
use egui_glium::egui_winit::egui::ViewportId;
use generator::{WorldGenerator, WorldGeneratorOptions};
use mesh::DefaultUniforms;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};
use world::World;

mod app;
mod camera;
mod chunk;
mod generator;
mod mesh;
mod quad;
mod transform;
mod utils;
mod world;

struct VoxelApp {
    window: Rc<Window>,
    is_cursor_hidden: bool,

    camera: Camera,
    camera_controller: CameraController,
    projection: Projection,
    default_shader: glium::Program,

    seed: String,
    world_size: [String; 3],
    world_generator_options: WorldGeneratorOptions,
    world: World,

    egui: egui_glium::EguiGlium,
}

impl AppBehaviour for VoxelApp {
    fn process_events(&mut self, event: Event<()>) -> bool {
        match event {
            Event::WindowEvent { event, .. } => {
                let _ = self.egui.on_event(&self.window.winit, &event);
                match event {
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                        ..
                    } => false,
                    WindowEvent::Resized(window_size) => {
                        self.projection
                            .resize(window_size.width as f32, window_size.height as f32);
                        true
                    }
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(key),
                                state,
                                ..
                            },
                        ..
                    } => {
                        if key == KeyCode::AltLeft && state == ElementState::Pressed {
                            self.is_cursor_hidden = false;
                        } else if key == KeyCode::AltLeft && state == ElementState::Released {
                            self.is_cursor_hidden = true;
                        }

                        self.camera_controller.process_keyboard(key, state);
                        true
                    }
                    _ => true,
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if self.is_cursor_hidden {
                    self.camera_controller
                        .process_mouse(delta.0 as f32, delta.1 as f32);
                }

                true
            }
            _ => true,
        }
    }

    fn update(&mut self, delta_time: std::time::Duration) {
        self.camera_controller
            .update_camera(&mut self.camera, delta_time.as_secs_f32());
    }

    fn render(&mut self, frame: &mut glium::Frame) {
        self.window.winit.set_cursor_visible(!self.is_cursor_hidden);

        self.world.draw(
            frame,
            &self.default_shader,
            DefaultUniforms {
                view_projection: (self.projection.matrix() * self.camera.view_matrix())
                    .to_cols_array_2d(),
                light_color: [1.0, 1.0, 1.0],
                light_position: [100.0, 100.0, 100.0],
            },
        );

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

                ui.separator();

                if ui
                    .add(egui::Button::new("Generate"))
                    .on_hover_ui(|ui| {
                        ui.label("Generate a new world with the given seed.");
                    })
                    .clicked()
                {
                    self.world_generator_options.seed = self.seed.parse().expect("to parse seed");
                    let world_generator = WorldGenerator::new(self.world_generator_options.clone());
                    self.world = World::new(&self.window, &world_generator);
                }
            });
        });

        self.egui.paint(&self.window.display, frame);
    }
}

impl VoxelApp {
    fn new(window: Rc<Window>, event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        window
            .winit
            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
            .or_else(|_| {
                window
                    .winit
                    .set_cursor_grab(winit::window::CursorGrabMode::Confined)
            })
            .expect("to lock cursor to window");
        window.winit.set_cursor_visible(false);

        let default_shader = glium::Program::from_source(
            &window.display,
            include_str!("shaders/shader.vert"),
            include_str!("shaders/shader.frag"),
            None,
        )
        .expect("to compile default shaders");

        let camera = Camera::new(glam::vec3(0.0, 0.0, 0.0), 0.0, 0.0);
        let camera_controller = CameraController::new(20.0, 0.5);

        let projection = {
            let window_size = window.winit.inner_size();
            Projection::new(
                window_size.width as f32 / window_size.height as f32,
                45.0,
                0.1,
                1000.0,
            )
        };

        let seed = "1337".to_string();
        let world_size = ["10".to_string(), "10".to_string(), "10".to_string()];
        let world_generator_options = WorldGeneratorOptions::builder()
            .seed(seed.parse().expect("to parse seed"))
            .chunk_size(CHUNK_SIZE)
            .world_size(glam::uvec3(
                world_size[0].parse().expect("to parse world size x"),
                world_size[1].parse().expect("to parse world size y"),
                world_size[2].parse().expect("to parse world size z"),
            ))
            .max_terrain_height(CHUNK_SIZE.y * 3)
            .dirt_layer_thickness(5)
            .sea_level(CHUNK_SIZE.y)
            .build();
        let world_generator = WorldGenerator::new(world_generator_options.clone());

        let world = World::new(&window, &world_generator);

        let egui = egui_glium::EguiGlium::new(
            ViewportId::ROOT,
            &window.display,
            &window.winit,
            event_loop,
        );

        Self {
            window,
            is_cursor_hidden: true,

            camera,
            camera_controller,
            projection,
            default_shader,

            seed,
            world_size,
            world_generator_options,
            world,

            egui,
        }
    }
}

fn main() {
    let mut app = App::new("Voxel", 1280, 720);

    let voxel_app = VoxelApp::new(app.window.clone(), &app.event_loop);
    app.run(voxel_app);
}
