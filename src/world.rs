use std::collections::HashMap;

use crossbeam_skiplist::SkipMap;
use glium::{DrawParameters, Surface};
use rayon::prelude::*;

use crate::{
    app::Window,
    chunk::{
        mesh::{Axis, Direction, Vertex},
        ChunkMesher, VoxelUniforms,
    },
    generator::WorldGenerator,
};

type ModelMatrix = [[f32; 4]; 4];
type NormalMatrix = [[f32; 3]; 3];

pub struct World {
    chunk_solid_buffers:
        HashMap<glam::UVec3, (glium::VertexBuffer<Vertex>, glium::IndexBuffer<u32>)>,
    chunk_transparent_buffers:
        HashMap<glam::UVec3, (glium::VertexBuffer<Vertex>, glium::IndexBuffer<u32>)>,
    chunk_uniforms: HashMap<glam::UVec3, (ModelMatrix, NormalMatrix)>,
}

impl World {
    pub fn new(window: &Window, generator: &WorldGenerator) -> Self {
        let now = std::time::Instant::now();
        let world = generator.generate_world();
        println!("World generation took: {:?}", now.elapsed());

        let now = std::time::Instant::now();
        let solid_meshes = SkipMap::new();
        let transparent_meshes = SkipMap::new();

        world
            .par_iter()
            .filter(|(_, chunk)| !chunk.is_empty())
            .for_each(|(&position, chunk)| {
                let mut neighbours = HashMap::new();

                for axis in [Axis::X, Axis::Y, Axis::Z] {
                    for direction in [Direction::Positive, Direction::Negative] {
                        let neighbour_position =
                            position.as_ivec3() + axis.get_normal(direction).as_ivec3();
                        let neighbour_position: Result<glam::UVec3, _> =
                            neighbour_position.try_into();
                        if let Ok(neighbour_position) = neighbour_position {
                            if let Some(neighbour) = world.get(&neighbour_position) {
                                neighbours.insert(neighbour_position, neighbour);
                            }
                        }
                    }
                }

                let (solid_mesh, transparent_mesh) = ChunkMesher::mesh(chunk, &neighbours);

                if let Some(solid_mesh) = solid_mesh {
                    solid_meshes.insert((position.x, position.y, position.z), solid_mesh);
                }

                if let Some(transparent_mesh) = transparent_mesh {
                    transparent_meshes
                        .insert((position.x, position.y, position.z), transparent_mesh);
                }
            });

        let mut chunk_solid_buffers = HashMap::new();
        let mut chunk_transparent_buffers = HashMap::new();
        let mut chunk_uniforms = HashMap::new();

        for mesh in solid_meshes.iter() {
            let position = {
                let position = mesh.key();
                glam::UVec3::new(position.0, position.1, position.2)
            };

            let mesh = mesh.value();

            chunk_solid_buffers.insert(
                position,
                (
                    mesh.vertex_buffer(&window.display).unwrap(),
                    mesh.index_buffer(&window.display).unwrap(),
                ),
            );

            let chunk = world.get(&position).unwrap();
            chunk_uniforms.insert(
                position,
                (
                    chunk.transform().model_matrix().to_cols_array_2d(),
                    chunk.transform().normal_matrix().to_cols_array_2d(),
                ),
            );
        }

        for mesh in transparent_meshes.iter() {
            let position = {
                let position = mesh.key();
                glam::UVec3::new(position.0, position.1, position.2)
            };

            let mesh = mesh.value();

            chunk_transparent_buffers.insert(
                position,
                (
                    mesh.vertex_buffer(&window.display).unwrap(),
                    mesh.index_buffer(&window.display).unwrap(),
                ),
            );
        }

        println!("Meshing took: {:?}", now.elapsed());

        Self {
            chunk_solid_buffers,
            chunk_transparent_buffers,
            chunk_uniforms,
        }
    }

    pub fn draw(
        &self,
        frame: &mut glium::Frame,
        shader: &glium::Program,
        uniforms: VoxelUniforms,
        draw_wireframe: bool,
    ) {
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
