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
    world_generator: WorldGenerator,
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
            egui::Window::new("Hello World").show(ctx, |ui| {
                ui.label("Hello World!");
                ui.label("This is a simple egui window.");
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

        let world_generator = WorldGenerator::new(
            WorldGeneratorOptions::builder()
                .seed(1337)
                .chunk_size(CHUNK_SIZE)
                .world_size(glam::UVec3::splat(10))
                .max_terrain_height(CHUNK_SIZE.y * 3)
                .dirt_layer_thickness(5)
                .sea_level(CHUNK_SIZE.y)
                .build(),
        );

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
            world_generator,
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
