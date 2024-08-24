#[macro_use]
extern crate glium;
use std::collections::HashMap;

use camera::{Camera, CameraController, Projection};
use chunk::{ChunkMesher, CHUNK_SIZE};
use generator::{WorldGenerator, WorldGeneratorOptions};
use glium::{
    winit::{
        event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent},
        keyboard::{KeyCode, PhysicalKey},
    },
    DrawParameters, Surface,
};
use num_traits::FromPrimitive;

mod camera;
mod chunk;
mod generator;
mod mesh;
mod quad;
mod transform;
mod utils;

fn main() {
    let event_loop = glium::winit::event_loop::EventLoop::builder()
        .build()
        .expect("event loop to be built");
    let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
        .with_title("Voxels")
        .with_inner_size(1280, 720)
        .build(&event_loop);

    window
        .set_cursor_grab(glium::winit::window::CursorGrabMode::Locked)
        .or_else(|_| window.set_cursor_grab(glium::winit::window::CursorGrabMode::Confined))
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
            .world_size(glam::UVec3::splat(5))
            .max_terrain_height(CHUNK_SIZE.y * 3)
            .dirt_layer_thickness(5)
            .sea_level(CHUNK_SIZE.y)
            .build(),
    );

    let world = world_generator.generate_world();

    let mut chunk_buffers = vec![];
    let mut chunk_uniforms = vec![];

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
        let buffers = mesh
            .as_opengl_buffers(&display)
            .expect("to create opengl buffers");

        chunk_buffers.push(buffers);
        chunk_uniforms.push((
            chunk.transform().model_matrix().to_cols_array_2d(),
            chunk.transform().normal_matrix().to_cols_array_2d(),
        ));
    }

    let mut last_frame_time = std::time::Instant::now();

    #[allow(deprecated)]
    event_loop
        .run(move |event, window_target| {
            match event {
                Event::WindowEvent { event, .. } => match event {
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

                        for ((vertices, indices), (model, normal)) in
                            chunk_buffers.iter().zip(chunk_uniforms.iter())
                        {
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
                                    ..Default::default()
                                },
                            )
                            .expect("to draw vertices");
                        }

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
                    } => camera_controller.process_keyboard(key, state),
                    _ => (),
                },
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta },
                    ..
                } => camera_controller.process_mouse(delta.0 as f32, delta.1 as f32),
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => (),
            };
        })
        .expect("to run event loop");
}
