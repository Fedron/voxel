#[macro_use]
extern crate glium;
use camera::Camera;
use glium::{
    winit::{
        event::{Event, WindowEvent},
        keyboard::KeyCode,
        platform::{pump_events::EventLoopExtPumpEvents, run_on_demand::EventLoopExtRunOnDemand},
    },
    Surface,
};
use mesh::Mesh;
use quad::QuadFace;

mod camera;
mod mesh;
mod quad;

const CAMERA_MOVE_SPEED: f32 = 10.0;
const MOUSE_SENSITIVITY: f32 = 0.2;

fn main() {
    let event_loop = glium::winit::event_loop::EventLoop::builder()
        .build()
        .expect("event loop to be built");
    let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
        .with_title("Voxels")
        .build(&event_loop);

    let quad: Mesh<4, 6> = QuadFace::Front.as_mesh(Default::default());
    let vertex_buffer =
        glium::VertexBuffer::new(&display, &quad.vertices).expect("to create vertex buffer");
    let indices = glium::index::IndexBuffer::new(
        &display,
        glium::index::PrimitiveType::TrianglesList,
        &quad.indices,
    )
    .expect("to create index buffer");

    let program = glium::Program::from_source(
        &display,
        include_str!("shaders/shader.vert"),
        include_str!("shaders/shader.frag"),
        None,
    )
    .expect("to compile shaders");

    let mut camera = Camera::new((0.0, 0.0, 5.0).into());

    let mut last_mouse_position = glam::vec2(0.0, 0.0);
    let mut is_first_mouse = true;

    #[allow(deprecated)]
    event_loop
        .run(move |event, window_target| {
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => window_target.exit(),
                    WindowEvent::RedrawRequested => {
                        let window_size = window.inner_size();
                        let projection = glam::Mat4::perspective_rh(
                            camera.aspect.to_radians(),
                            window_size.width as f32 / window_size.height as f32,
                            0.1,
                            1000.0,
                        );
                        let view_proj = (projection * camera.view_matrix()).to_cols_array_2d();

                        let mut frame = display.draw();
                        frame.clear_color(1.0, 0.0, 1.0, 1.0);
                        frame
                            .draw(
                                &vertex_buffer,
                                &indices,
                                &program,
                                &uniform! { view_proj: view_proj},
                                &Default::default(),
                            )
                            .expect("to draw vertices");
                        frame.finish().expect("to finish drawing");
                    }
                    WindowEvent::Resized(window_size) => {
                        display.resize(window_size.into());
                    }
                    WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                        glium::winit::keyboard::PhysicalKey::Code(key) => match key {
                            KeyCode::Escape => window_target.exit(),
                            KeyCode::KeyW => camera.process_movement(
                                camera::MoveDirection::Forward,
                                CAMERA_MOVE_SPEED,
                            ),
                            KeyCode::KeyA => camera
                                .process_movement(camera::MoveDirection::Left, CAMERA_MOVE_SPEED),
                            KeyCode::KeyS => camera.process_movement(
                                camera::MoveDirection::Backward,
                                CAMERA_MOVE_SPEED,
                            ),
                            KeyCode::KeyD => camera
                                .process_movement(camera::MoveDirection::Right, CAMERA_MOVE_SPEED),
                            KeyCode::Space => camera
                                .process_movement(camera::MoveDirection::Up, CAMERA_MOVE_SPEED),
                            KeyCode::ShiftLeft => camera
                                .process_movement(camera::MoveDirection::Down, CAMERA_MOVE_SPEED),
                            _ => (),
                        },
                        _ => (),
                    },
                    WindowEvent::CursorMoved { position, .. } => {
                        if is_first_mouse {
                            last_mouse_position.x = position.x as f32;
                            last_mouse_position.y = position.y as f32;
                            is_first_mouse = false;
                        }

                        let x_offset = position.x as f32 - last_mouse_position.x;
                        let y_offset = last_mouse_position.y - position.y as f32;

                        last_mouse_position.x = position.x as f32;
                        last_mouse_position.y = position.y as f32;

                        camera.process_mouse(
                            x_offset * MOUSE_SENSITIVITY,
                            y_offset * MOUSE_SENSITIVITY,
                        );
                    }
                    WindowEvent::MouseWheel { delta, .. } => match delta {
                        glium::winit::event::MouseScrollDelta::LineDelta(_, y_offset) => {
                            camera.aspect -= y_offset;
                            camera.aspect = camera.aspect.clamp(1.0, 45.0);
                        }
                        _ => {}
                    },
                    _ => (),
                },
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => (),
            };
        })
        .expect("to run event loop");
}
