#[macro_use]
extern crate glium;
use std::rc::Rc;

use app::{App, AppBehaviour, Window};
use camera::{Camera, CameraController, Projection};
use chunk::{VoxelUniforms, CHUNK_SIZE};
use generator::{WorldGenerator, WorldGeneratorOptions};
use glium::Surface;
use sky_dome::SkyDome;
use ui::WorldGeneratorUi;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};
use world::World;

mod app;
mod camera;
mod chunk;
mod generator;
mod sky_dome;
mod transform;
mod ui;
mod utils;
mod world;

struct VoxelApp {
    window: Rc<Window>,
    is_cursor_hidden: bool,

    camera: Camera,
    camera_controller: CameraController,
    projection: Projection,

    sky_dome: SkyDome,
    voxel_shader: glium::Program,
    world: World,
    world_generator_ui: WorldGeneratorUi,

    render_wireframe: bool,
}

impl AppBehaviour for VoxelApp {
    fn process_events(&mut self, event: Event<()>) -> bool {
        match event {
            Event::WindowEvent { event, .. } => {
                self.world_generator_ui.process_events(&event);
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

                        if key == KeyCode::F3 && state == ElementState::Pressed {
                            self.render_wireframe = !self.render_wireframe;
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

        self.sky_dome.position = self.camera.position - glam::vec3(0.0, 200.0, 0.0);

        if self.world_generator_ui.should_generate_world {
            self.world_generator_ui.should_generate_world = false;

            let world_generator =
                WorldGenerator::new(self.world_generator_ui.world_generator_options.clone());
            self.world = World::new(&self.window, &world_generator);
        }
    }

    fn render(&mut self, frame: &mut glium::Frame) {
        self.window.winit.set_cursor_visible(!self.is_cursor_hidden);

        frame.clear_color_srgb(0.71, 0.85, 0.90, 1.0);

        let view_projection = self.projection.matrix() * self.camera.view_matrix();

        self.world.draw(
            frame,
            &self.voxel_shader,
            VoxelUniforms {
                view_projection: view_projection.to_cols_array_2d(),
                light_color: [1.0, 1.0, 1.0],
                light_position: [100.0, 100.0, 100.0],
            },
            self.render_wireframe,
        );

        self.sky_dome.draw(frame, view_projection);

        self.world_generator_ui.render(frame);
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

        let voxel_shader = glium::Program::from_source(
            &window.display,
            include_str!("shaders/voxel.vert"),
            include_str!("shaders/voxel.frag"),
            None,
        )
        .expect("to compile default shaders");

        let camera = Camera::new(
            glam::vec3(
                CHUNK_SIZE.x as f32 * -5.0,
                CHUNK_SIZE.y as f32 * 5.0,
                CHUNK_SIZE.z as f32 * 2.5,
            ),
            0.0,
            -30.0f32.to_radians(),
        );
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

        let sky_dome = SkyDome::new(&window.display, 20, 20, 500.0);

        let world_generator_options = WorldGeneratorOptions::builder()
            .seed(1337)
            .chunk_size(CHUNK_SIZE)
            .world_size(glam::UVec3::splat(2))
            .max_terrain_height(CHUNK_SIZE.y * 3)
            .dirt_layer_thickness(5)
            .sea_level(CHUNK_SIZE.y)
            .build();
        let world_generator = WorldGenerator::new(world_generator_options.clone());

        let world = World::new(&window, &world_generator);

        let world_generator_ui =
            WorldGeneratorUi::new(world_generator_options, window.clone(), event_loop);

        Self {
            window,
            is_cursor_hidden: true,

            camera,
            camera_controller,
            projection,

            sky_dome,
            voxel_shader,
            world,
            world_generator_ui,

            render_wireframe: false,
        }
    }
}

fn main() {
    let mut app = App::new("Voxel", 1920, 1080);

    let voxel_app = VoxelApp::new(app.window.clone(), &app.event_loop);
    app.run(voxel_app);
}
