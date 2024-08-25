#[macro_use]
extern crate glium;
use std::{collections::HashMap, rc::Rc};

use app::{App, AppBehaviour, Window};
use camera::{Camera, CameraController, Projection};
use chunk::{ChunkMesher, CHUNK_SIZE};
use egui_glium::egui_winit::egui::ViewportId;
use generator::{WorldGenerator, WorldGeneratorOptions};
use glium::{DrawParameters, Surface};
use num_traits::FromPrimitive;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

mod app;
mod camera;
mod chunk;
mod generator;
mod mesh;
mod quad;
mod transform;
mod utils;

type ModelMatrix = [[f32; 4]; 4];
type NormalMatrix = [[f32; 3]; 3];

struct VoxelApp {
    window: Rc<Window>,
    is_cursor_hidden: bool,

    camera: Camera,
    camera_controller: CameraController,
    projection: Projection,

    program: glium::Program,
    chunk_solid_buffers:
        HashMap<glam::UVec3, (glium::VertexBuffer<mesh::Vertex>, glium::IndexBuffer<u32>)>,
    chunk_transparent_buffers:
        HashMap<glam::UVec3, (glium::VertexBuffer<mesh::Vertex>, glium::IndexBuffer<u32>)>,
    chunk_uniforms: HashMap<glam::UVec3, (ModelMatrix, NormalMatrix)>,

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

        let view_proj = (self.projection.matrix() * self.camera.view_matrix()).to_cols_array_2d();

        let light_color: [f32; 3] = [1.0, 1.0, 1.0];
        let light_position: [f32; 3] = [100.0, 100.0, 100.0];

        for (position, (vertices, indices)) in self.chunk_solid_buffers.iter() {
            let (model, normal) = self.chunk_uniforms.get(position).unwrap();
            frame
                .draw(
                    vertices,
                    indices,
                    &self.program,
                    &uniform! {
                        view_proj: view_proj,
                        model: *model,
                        normal_matrix: *normal,
                        light_color: light_color,
                        light_position: light_position
                    },
                    &DrawParameters {
                        depth: glium::Depth {
                            test: glium::draw_parameters::DepthTest::IfLess,
                            write: true,
                            ..Default::default()
                        },
                        backface_culling:
                            glium::draw_parameters::BackfaceCullingMode::CullClockwise,
                        blend: glium::Blend::alpha_blending(),
                        ..Default::default()
                    },
                )
                .expect("to draw vertices");
        }

        for (position, (vertices, indices)) in self.chunk_transparent_buffers.iter() {
            let (model, normal) = self.chunk_uniforms.get(position).unwrap();
            frame
                .draw(
                    vertices,
                    indices,
                    &self.program,
                    &uniform! {
                        view_proj: view_proj,
                        model: *model,
                        normal_matrix: *normal,
                        light_color: light_color,
                        light_position: light_position
                    },
                    &DrawParameters {
                        depth: glium::Depth {
                            test: glium::draw_parameters::DepthTest::IfLess,
                            write: true,
                            ..Default::default()
                        },
                        backface_culling:
                            glium::draw_parameters::BackfaceCullingMode::CullClockwise,
                        blend: glium::Blend::alpha_blending(),
                        ..Default::default()
                    },
                )
                .expect("to draw vertices");
        }

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

        let program = glium::Program::from_source(
            &window.display,
            include_str!("shaders/shader.vert"),
            include_str!("shaders/shader.frag"),
            None,
        )
        .expect("to compile shaders");

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

        let world = world_generator.generate_world();

        let mut chunk_solid_buffers = HashMap::new();
        let mut chunk_transparent_buffers = HashMap::new();
        let mut chunk_uniforms = HashMap::new();

        for (&position, chunk) in world.iter() {
            let mut neighbours = HashMap::new();
            for i in 0..6 {
                let neighbour_position = position.saturating_add_signed(
                    quad::QuadFace::from_i64(i as i64)
                        .expect("to convert primitive to quad face enum")
                        .into(),
                );
                if let Some(neighbour) = world.get(&neighbour_position) {
                    neighbours.insert(neighbour_position, neighbour);
                }
            }

            let mesh = ChunkMesher::mesh(chunk, neighbours);

            chunk_uniforms.insert(
                position,
                (
                    chunk.transform().model_matrix().to_cols_array_2d(),
                    chunk.transform().normal_matrix().to_cols_array_2d(),
                ),
            );

            chunk_solid_buffers.insert(
                position,
                mesh.solid
                    .as_opengl_buffers(&window.display)
                    .expect("to create opengl buffers"),
            );

            if let Some(transparent) = mesh.transparent {
                chunk_transparent_buffers.insert(
                    position,
                    transparent
                        .as_opengl_buffers(&window.display)
                        .expect("to create opengl buffers"),
                );
            }
        }

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

            program,
            chunk_solid_buffers,
            chunk_transparent_buffers,
            chunk_uniforms,

            egui,
        }
    }
}

fn main() {
    let mut app = App::new("Voxel", 1280, 720);

    let voxel_app = VoxelApp::new(app.window.clone(), &app.event_loop);
    app.run(voxel_app);
}
