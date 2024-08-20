pub struct Camera {
    pub eye: glam::Vec3,
    pub target: glam::Vec3,
    pub up: glam::Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub near_plane: f32,
    pub far_plane: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> glam::Mat4 {
        let view = glam::Mat4::look_at_rh(self.eye, self.target, self.up);
        let projection =
            glam::Mat4::perspective_rh(self.fovy, self.aspect, self.near_plane, self.far_plane);

        projection * view
    }
}
