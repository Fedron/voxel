use std::time::Duration;

use glam::{Mat3, Mat4, Quat, Vec3};
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

const MOVE_SPEED: f32 = 3.0;
const ANGLE_PER_POINT: f32 = 0.001745;

const FORWARD_KEYCODE: KeyCode = KeyCode::KeyW;
const BACKWARD_KEYCODE: KeyCode = KeyCode::KeyS;
const RIGHT_KEYCODE: KeyCode = KeyCode::KeyD;
const LEFT_KEYCODE: KeyCode = KeyCode::KeyA;
const UP_KEYCODE: KeyCode = KeyCode::Space;
const DOWN_KEYCODE: KeyCode = KeyCode::ControlLeft;

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: Vec3,
    pub direction: Vec3,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub fn new(
        position: Vec3,
        direction: Vec3,
        fov: f32,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        Self {
            position,
            direction: direction.normalize(),
            fov,
            aspect_ratio,
            z_near,
            z_far,
        }
    }

    pub fn update(self, controls: &CameraControls, delta_time: Duration) -> Self {
        let delta_time = delta_time.as_secs_f32();
        let side = self.direction.cross(glam::Vec3::Y);

        let new_direction = {
            let side_rot = Quat::from_axis_angle(side, controls.cursor_delta[1] * ANGLE_PER_POINT);
            let y_rot = Quat::from_rotation_y(-controls.cursor_delta[0] * ANGLE_PER_POINT);
            let rot = Mat3::from_quat(side_rot * y_rot);

            (rot * self.direction).normalize()
        };

        let mut direction = Vec3::ZERO;

        if controls.go_forward {
            direction += new_direction;
        }
        if controls.go_backward {
            direction -= new_direction;
        }
        if controls.strafe_right {
            direction += side;
        }
        if controls.strafe_left {
            direction -= side;
        }
        if controls.go_up {
            direction += glam::Vec3::Y;
        }
        if controls.go_down {
            direction -= glam::Vec3::Y;
        }

        let direction = if direction.length_squared() == 0.0 {
            direction
        } else {
            direction.normalize()
        };

        Self {
            position: self.position + direction * MOVE_SPEED * delta_time,
            direction: new_direction,
            ..self
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.direction, glam::Vec3::Y)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fov.to_radians(),
            self.aspect_ratio,
            self.z_near,
            self.z_far,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CameraControls {
    pub go_forward: bool,
    pub go_backward: bool,
    pub strafe_right: bool,
    pub strafe_left: bool,
    pub go_up: bool,
    pub go_down: bool,
    pub cursor_delta: [f32; 2],
}

impl Default for CameraControls {
    fn default() -> Self {
        Self {
            go_forward: false,
            go_backward: false,
            strafe_right: false,
            strafe_left: false,
            go_up: false,
            go_down: false,
            cursor_delta: [0.0; 2],
        }
    }
}

impl CameraControls {
    pub fn reset(self) -> Self {
        Self {
            cursor_delta: [0.0; 2],
            ..self
        }
    }

    pub fn handle_event(self, event: &Event<()>) -> Self {
        let mut new_state = self;

        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(code),
                                state,
                                ..
                            },
                        ..
                    } => match *code {
                        FORWARD_KEYCODE => new_state.go_forward = *state == ElementState::Pressed,
                        BACKWARD_KEYCODE => new_state.go_backward = *state == ElementState::Pressed,
                        RIGHT_KEYCODE => new_state.strafe_right = *state == ElementState::Pressed,
                        LEFT_KEYCODE => new_state.strafe_left = *state == ElementState::Pressed,
                        UP_KEYCODE => new_state.go_up = *state == ElementState::Pressed,
                        DOWN_KEYCODE => new_state.go_down = *state == ElementState::Pressed,
                        _ => (),
                    },
                    _ => {}
                };
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta: (x, y) },
                ..
            } => {
                let x = *x as f32;
                let y = *y as f32;
                new_state.cursor_delta = [self.cursor_delta[0] + x, self.cursor_delta[1] + y];
            }
            _ => (),
        }

        new_state
    }
}
