#[macro_use]
extern crate glium;
use camera::{Camera, CameraController, Projection};
use chunk::{ChunkMesher, CHUNK_SIZE};
use generator::WorldGenerator;
use glium::{
    winit::{
        event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent},
        keyboard::{KeyCode, PhysicalKey},
    },
    DrawParameters, Surface,
};

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
    let mut camera_controller = CameraController::new(10.0, 0.5);

    let mut projection = {
        let window_size = window.inner_size();
        Projection::new(
            window_size.width as f32 / window_size.height as f32,
            45.0,
            0.1,
            1000.0,
        )
    };

    let world_generator = WorldGenerator::builder()
        .seed(1337)
        .chunk_size(CHUNK_SIZE)
        .max_world_height(CHUNK_SIZE.y)
        .build();

    let chunk = world_generator.generate_chunk(glam::uvec3(0, 0, 0));
    let chunk_mesh = ChunkMesher::mesh(&chunk);

    let chunk_model = chunk.transform().model_matrix().to_cols_array_2d();
    let chunk_normal = chunk.transform().normal_matrix().to_cols_array_2d();

    let vertex_buffer =
        glium::VertexBuffer::new(&display, &chunk_mesh.vertices).expect("to create vertex buffer");
    let indices = glium::index::IndexBuffer::new(
        &display,
        glium::index::PrimitiveType::TrianglesList,
        &chunk_mesh.indices,
    )
    .expect("to create index buffer");

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
                        let light_position: [f32; 3] = [20.0, 20.0, 20.0];

                        let mut frame = display.draw();
                        frame.clear_color(0.0, 0.45, 0.74, 1.0);
                        frame
                            .draw(
                                &vertex_buffer,
                                &indices,
                                &program,
                                &uniform! {
                                    view_proj: view_proj,
                                    model: chunk_model,
                                    normal_matrix: chunk_normal,
                                    light_color: light_color,
                                    light_position: light_position
                                },
                                &DrawParameters {
                                    backface_culling:
                                        glium::draw_parameters::BackfaceCullingMode::CullClockwise,
                                    ..Default::default()
                                },
                            )
                            .expect("to draw vertices");
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
