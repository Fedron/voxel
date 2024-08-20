pub enum MoveDirection {
    Forward,
    Backward,
    Left,
    Right,
    Up,
    Down,
}

pub struct Camera {
    position: glam::Vec3,
    front: glam::Vec3,
    up: glam::Vec3,
    right: glam::Vec3,

    yaw: f32,
    pitch: f32,
    pub aspect: f32,
}

impl Camera {
    pub fn new(position: glam::Vec3) -> Self {
        Self {
            position,
            front: glam::Vec3::Z,
            up: glam::Vec3::Y,
            right: glam::Vec3::X,

            yaw: -90.0,
            pitch: 0.0,
            aspect: 45.0,
        }
    }

    pub fn view_matrix(&self) -> glam::Mat4 {
        glam::Mat4::look_at_rh(self.position, self.position + self.front, self.up)
    }

    pub fn process_movement(&mut self, direction: MoveDirection, velocity: f32) {
        match direction {
            MoveDirection::Forward => self.position += self.front * velocity,
            MoveDirection::Backward => self.position -= self.front * velocity,
            MoveDirection::Left => self.position -= self.right * velocity,
            MoveDirection::Right => self.position += self.right * velocity,
            MoveDirection::Up => self.position += self.up * velocity,
            MoveDirection::Down => self.position -= self.up * velocity,
        }
    }

    pub fn process_mouse(&mut self, x_offset: f32, y_offset: f32) {
        self.yaw += x_offset;
        self.pitch += y_offset;

        if self.pitch > 89.0 {
            self.pitch = 89.0;
        }
        if self.pitch < -89.0 {
            self.pitch = -89.0;
        }

        self.update_vectors();
    }

    fn update_vectors(&mut self) {
        self.front.x = self.yaw.to_radians().cos() * self.pitch.to_radians().cos();
        self.front.y = self.pitch.to_radians().sin();
        self.front.z = self.yaw.to_radians().sin() * self.pitch.to_radians().cos();

        self.right = self.front.cross(glam::Vec3::Y).normalize();
        self.up = self.right.cross(self.front).normalize();
    }
}
