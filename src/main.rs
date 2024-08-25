#[macro_use]
extern crate glium;
use std::collections::HashMap;

use camera::{Camera, CameraController, Projection};
use chunk::{ChunkMesher, CHUNK_SIZE};
use egui_glium::egui_winit::egui::ViewportId;
use generator::{WorldGenerator, WorldGeneratorOptions};
use glium::{DrawParameters, Surface};
use num_traits::FromPrimitive;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
};

mod camera;
mod chunk;
mod generator;
mod mesh;
mod quad;
mod transform;
mod utils;

fn main() {
    let event_loop = EventLoop::new().expect("to create event loop");
    let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
        .with_title("Voxels")
        .with_inner_size(1280, 720)
        .build(&event_loop);

    let mut gui = egui_glium::EguiGlium::new(ViewportId::ROOT, &display, &window, &event_loop);

    window
        .set_cursor_grab(winit::window::CursorGrabMode::Locked)
        .or_else(|_| window.set_cursor_grab(winit::window::CursorGrabMode::Confined))
        .expect("to lock cursor to window");
    window.set_cursor_visible(false);

    let program = glium::Program::from_source(
        &display,
        include_str!("shaders/shader.vert"),
        include_str!("shaders/shader.frag"),
        None,
    )
    .expect("to compile shaders");

    let mut camera = Camera::new(glam::vec3(0.0, 0.0, 0.0), 0.0, 0.0);
    let mut camera_controller = CameraController::new(20.0, 0.5);

    let mut projection = {
        let window_size = window.inner_size();
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
                .as_opengl_buffers(&display)
                .expect("to create opengl buffers"),
        );

        if let Some(transparent) = mesh.transparent {
            chunk_transparent_buffers.insert(
                position,
                transparent
                    .as_opengl_buffers(&display)
                    .expect("to create opengl buffers"),
            );
        }
    }

    let mut last_frame_time = std::time::Instant::now();
    let mut is_holding_alt = false;

    #[allow(deprecated)]
    event_loop
        .run(move |event, window_target| {
            match event {
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                                    ..
                                },
                            ..
                        } => window_target.exit(),
                        WindowEvent::RedrawRequested => {
                            let current_time = std::time::Instant::now();
                            let delta_time = current_time.duration_since(last_frame_time);
                            last_frame_time = current_time;

                            camera_controller.update_camera(&mut camera, delta_time.as_secs_f32());

                            let view_proj =
                                (projection.matrix() * camera.view_matrix()).to_cols_array_2d();

                            let light_color: [f32; 3] = [1.0, 1.0, 1.0];
                            let light_position: [f32; 3] = [100.0, 100.0, 100.0];

                            let mut frame = display.draw();
                            frame.clear_color_and_depth((0.0, 0.45, 0.74, 1.0), 1.0);

                            for (position, (vertices, indices)) in chunk_solid_buffers.iter() {
                                let (model, normal) = chunk_uniforms.get(position).unwrap();
                                frame
                            .draw(
                                vertices,
                                indices,
                                &program,
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

                            for (position, (vertices, indices)) in chunk_transparent_buffers.iter()
                            {
                                let (model, normal) = chunk_uniforms.get(position).unwrap();
                                frame
                            .draw(
                                vertices,
                                indices,
                                &program,
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

                            gui.run(&window, |ctx| {
                                egui::Window::new("Hello World").show(ctx, |ui| {
                                    ui.label("Hello World!");
                                    ui.label("This is a simple egui window.");
                                });
                            });

                            gui.paint(&display, &mut frame);

                            frame.finish().expect("to finish drawing");
                        }
                        WindowEvent::Resized(window_size) => {
                            display.resize(window_size.into());
                            projection.resize(window_size.width as f32, window_size.height as f32);
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
                                is_holding_alt = true;
                                window.set_cursor_visible(true);
                            } else if key == KeyCode::AltLeft && state == ElementState::Released {
                                is_holding_alt = false;
                                window.set_cursor_visible(false);
                            }

                            camera_controller.process_keyboard(key, state);
                        }
                        _ => (),
                    }
                    let _ = gui.on_event(&window, &event);
                }
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta },
                    ..
                } => {
                    if !is_holding_alt {
                        camera_controller.process_mouse(delta.0 as f32, delta.1 as f32);
                    }
                }
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => (),
            };
        })
        .expect("to run event loop");
}
