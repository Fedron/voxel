use std::collections::HashMap;

use glium::{DrawParameters, Surface};

use crate::{
    app::Window,
    chunk::{
        mesh::{Axis, Direction, Vertex},
        ChunkMesher, VoxelUniforms, CHUNK_SIZE,
    },
    generator::WorldGenerator,
};

type ModelMatrix = [[f32; 4]; 4];
type NormalMatrix = [[f32; 3]; 3];

pub struct World {
    chunk_buffers: HashMap<glam::UVec3, (glium::VertexBuffer<Vertex>, glium::IndexBuffer<u32>)>,
    chunk_uniforms: HashMap<glam::UVec3, (ModelMatrix, NormalMatrix)>,
    chunk_greedy_buffers:
        HashMap<glam::UVec3, (glium::VertexBuffer<Vertex>, glium::IndexBuffer<u32>)>,
    chunk_greedy_uniforms: HashMap<glam::UVec3, (ModelMatrix, NormalMatrix)>,
}

impl World {
    pub fn new(window: &Window, generator: &WorldGenerator) -> Self {
        let world = generator.generate_world();

        let mut chunk_buffers = HashMap::new();
        let mut chunk_uniforms = HashMap::new();
        let mut chunk_greedy_buffers = HashMap::new();
        let mut chunk_greedy_uniforms = HashMap::new();

        for (&position, chunk) in world.iter() {
            let mut neighbours = HashMap::new();

            for axis in [Axis::X, Axis::Y, Axis::Z] {
                for direction in [Direction::Positive, Direction::Negative] {
                    let neighbour_position =
                        position.as_ivec3() + axis.get_normal(direction).as_ivec3();
                    let neighbour_position: Result<glam::UVec3, _> = neighbour_position.try_into();
                    if let Ok(neighbour_position) = neighbour_position {
                        if let Some(neighbour) = world.get(&neighbour_position) {
                            neighbours.insert(neighbour_position, neighbour);
                        }
                    }
                }
            }

            let mesh = ChunkMesher::mesh(chunk, neighbours.clone());
            let greedy_mesh = ChunkMesher::greedy_mesh(chunk, &neighbours);

            let mut transform = chunk.transform();
            transform.position.x += CHUNK_SIZE.x as f32 * 6.0;
            chunk_uniforms.insert(
                position,
                (
                    transform.model_matrix().to_cols_array_2d(),
                    transform.normal_matrix().to_cols_array_2d(),
                ),
            );

            chunk_buffers.insert(
                position,
                (
                    mesh.vertex_buffer(&window.display).unwrap(),
                    mesh.index_buffer(&window.display).unwrap(),
                ),
            );

            chunk_greedy_uniforms.insert(
                position,
                (
                    chunk.transform().model_matrix().to_cols_array_2d(),
                    chunk.transform().normal_matrix().to_cols_array_2d(),
                ),
            );

            chunk_greedy_buffers.insert(
                position,
                (
                    greedy_mesh.vertex_buffer(&window.display).unwrap(),
                    greedy_mesh.index_buffer(&window.display).unwrap(),
                ),
            );
        }

        Self {
            chunk_buffers,
            chunk_uniforms,
            chunk_greedy_buffers,
            chunk_greedy_uniforms,
        }
    }

    pub fn draw(
        &self,
        frame: &mut glium::Frame,
        shader: &glium::Program,
        uniforms: VoxelUniforms,
        draw_wireframe: bool,
    ) {
        for (position, (vertices, indices)) in self.chunk_buffers.iter() {
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
                        polygon_mode: if draw_wireframe {
                            glium::draw_parameters::PolygonMode::Line
                        } else {
                            glium::draw_parameters::PolygonMode::Fill
                        },
                        depth: glium::Depth {
                            test: glium::draw_parameters::DepthTest::IfLess,
                            write: true,
                            ..Default::default()
                        },
                        backface_culling:
                            glium::draw_parameters::BackfaceCullingMode::CullCounterClockwise,
                        blend: glium::Blend::alpha_blending(),
                        ..Default::default()
                    },
                )
                .expect("to draw vertices");
        }

        for (position, (vertices, indices)) in self.chunk_greedy_buffers.iter() {
            let (model, normal) = self.chunk_greedy_uniforms.get(position).unwrap();
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
                        polygon_mode: if draw_wireframe {
                            glium::draw_parameters::PolygonMode::Line
                        } else {
                            glium::draw_parameters::PolygonMode::Fill
                        },
                        depth: glium::Depth {
                            test: glium::draw_parameters::DepthTest::IfLess,
                            write: true,
                            ..Default::default()
                        },
                        backface_culling:
                            glium::draw_parameters::BackfaceCullingMode::CullCounterClockwise,
                        blend: glium::Blend::alpha_blending(),
                        ..Default::default()
                    },
                )
                .expect("to draw vertices");
        }
    }
}
