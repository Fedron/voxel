/// Raw float array for a 4x4 matrix.
pub type Matrix4x4 = [[f32; 4]; 4];
/// Raw float array for a 3x3 matrix.
pub type Matrix3x3 = [[f32; 3]; 3];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

impl Transform {
    pub fn model_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    pub fn normal_matrix(&self) -> glam::Mat3 {
        glam::Mat3::from_mat4(self.model_matrix().inverse().transpose())
    }
}
