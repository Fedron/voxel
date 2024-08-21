use crate::mesh::{Mesh, Vertex};

pub struct QuadFaceOptions {
    pub half_size: f32,
    pub base_position: [f32; 3],
    pub base_index: u32,
    pub color: [f32; 3],
}

impl Default for QuadFaceOptions {
    fn default() -> Self {
        Self {
            half_size: 0.5,
            base_position: [0.0; 3],
            base_index: 0,
            color: [0.0, 1.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, Copy, num_derive::FromPrimitive)]
pub enum QuadFace {
    Front,
    Back,
    Top,
    Bottom,
    Left,
    Right,
}

impl Into<glam::IVec3> for QuadFace {
    fn into(self) -> glam::IVec3 {
        match self {
            QuadFace::Front => glam::IVec3::Z,
            QuadFace::Back => glam::IVec3::NEG_Z,
            QuadFace::Top => glam::IVec3::Y,
            QuadFace::Bottom => glam::IVec3::NEG_Y,
            QuadFace::Left => glam::IVec3::NEG_X,
            QuadFace::Right => glam::IVec3::X,
        }
    }
}

impl QuadFace {
    pub fn as_mesh(&self, options: QuadFaceOptions) -> Mesh {
        let indices = vec![
            options.base_index,
            options.base_index + 1,
            options.base_index + 3,
            options.base_index + 1,
            options.base_index + 2,
            options.base_index + 3,
        ];
        let create_vertex = |quad_vertex: QuadVertex| {
            let position: [f32; 3] = quad_vertex.into();
            Vertex {
                position: [
                    position[0] * options.half_size + options.base_position[0],
                    position[1] * options.half_size + options.base_position[1],
                    position[2] * options.half_size + options.base_position[2],
                ],
                color: options.color,
            }
        };

        match self {
            QuadFace::Front => Mesh {
                vertices: vec![
                    create_vertex(QuadVertex::FrontTopLeft),
                    create_vertex(QuadVertex::FrontTopRight),
                    create_vertex(QuadVertex::FrontBottomRight),
                    create_vertex(QuadVertex::FrontBottomLeft),
                ],
                indices,
            },
            QuadFace::Back => Mesh {
                vertices: vec![
                    create_vertex(QuadVertex::BackTopRight),
                    create_vertex(QuadVertex::BackTopLeft),
                    create_vertex(QuadVertex::BackBottomLeft),
                    create_vertex(QuadVertex::BackBottomRight),
                ],
                indices,
            },
            QuadFace::Top => Mesh {
                vertices: vec![
                    create_vertex(QuadVertex::BackTopLeft),
                    create_vertex(QuadVertex::BackTopRight),
                    create_vertex(QuadVertex::FrontTopRight),
                    create_vertex(QuadVertex::FrontTopLeft),
                ],
                indices,
            },
            QuadFace::Bottom => Mesh {
                vertices: vec![
                    create_vertex(QuadVertex::BackBottomLeft),
                    create_vertex(QuadVertex::BackBottomRight),
                    create_vertex(QuadVertex::FrontBottomRight),
                    create_vertex(QuadVertex::FrontBottomLeft),
                ],
                indices,
            },
            QuadFace::Left => Mesh {
                vertices: vec![
                    create_vertex(QuadVertex::BackTopLeft),
                    create_vertex(QuadVertex::FrontTopLeft),
                    create_vertex(QuadVertex::FrontBottomLeft),
                    create_vertex(QuadVertex::BackBottomLeft),
                ],
                indices,
            },
            QuadFace::Right => Mesh {
                vertices: vec![
                    create_vertex(QuadVertex::FrontTopRight),
                    create_vertex(QuadVertex::BackTopRight),
                    create_vertex(QuadVertex::BackBottomRight),
                    create_vertex(QuadVertex::FrontBottomRight),
                ],
                indices,
            },
        }
    }
}

enum QuadVertex {
    FrontTopLeft,
    FrontTopRight,
    FrontBottomLeft,
    FrontBottomRight,
    BackTopLeft,
    BackTopRight,
    BackBottomLeft,
    BackBottomRight,
}

impl Into<[f32; 3]> for QuadVertex {
    fn into(self) -> [f32; 3] {
        match self {
            QuadVertex::FrontTopLeft => [-1.0, 1.0, -1.0],
            QuadVertex::FrontTopRight => [1.0, 1.0, -1.0],
            QuadVertex::FrontBottomLeft => [-1.0, -1.0, -1.0],
            QuadVertex::FrontBottomRight => [1.0, -1.0, -1.0],
            QuadVertex::BackTopLeft => [-1.0, 1.0, 1.0],
            QuadVertex::BackTopRight => [1.0, 1.0, 1.0],
            QuadVertex::BackBottomLeft => [-1.0, -1.0, 1.0],
            QuadVertex::BackBottomRight => [1.0, -1.0, 1.0],
        }
    }
}
