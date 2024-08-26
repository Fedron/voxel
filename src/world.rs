use std::collections::HashMap;

use glium::{DrawParameters, Surface};
use num_traits::FromPrimitive;

use crate::{
    app::Window,
    chunk::{ChunkMesher, VoxelUniforms, VoxelVertex},
    generator::WorldGenerator,
    quad::QuadFace,
};

type ModelMatrix = [[f32; 4]; 4];
type NormalMatrix = [[f32; 3]; 3];

pub struct World {
    chunk_solid_buffers:
        HashMap<glam::UVec3, (glium::VertexBuffer<VoxelVertex>, glium::IndexBuffer<u32>)>,
    chunk_transparent_buffers:
        HashMap<glam::UVec3, (glium::VertexBuffer<VoxelVertex>, glium::IndexBuffer<u32>)>,
    chunk_uniforms: HashMap<glam::UVec3, (ModelMatrix, NormalMatrix)>,
}

impl World {
    pub fn new(window: &Window, generator: &WorldGenerator) -> Self {
        let world = generator.generate_world();

        let mut chunk_solid_buffers = HashMap::new();
        let mut chunk_transparent_buffers = HashMap::new();
        let mut chunk_uniforms = HashMap::new();

        for (&position, chunk) in world.iter() {
            let mut neighbours = HashMap::new();
            for i in 0..6 {
                let neighbour_position = position.saturating_add_signed(
                    QuadFace::from_i64(i as i64)
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

        Self {
            chunk_solid_buffers,
            chunk_transparent_buffers,
            chunk_uniforms,
        }
    }

    pub fn draw(&self, frame: &mut glium::Frame, shader: &glium::Program, uniforms: VoxelUniforms) {
        for (position, (vertices, indices)) in self.chunk_solid_buffers.iter() {
            let (model, normal) = self.chunk_uniforms.get(position).unwrap();
            frame
                .draw(
                    vertices,
                    indices,
                    &shader,
                    &uniform! {
                        view_proj: uniforms.view_projection,
                        model: *model,
                        normal_matrix: *normal,
                        light_color: uniforms.light_color,
                        light_position: uniforms.light_position
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
                    &shader,
                    &uniform! {
                        view_proj: uniforms.view_projection,
                        model: *model,
                        normal_matrix: *normal,
                        light_color: uniforms.light_color,
                        light_position: uniforms.light_position
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
    }
}
