use std::collections::{HashMap, HashSet};

use mesh::{Axis, Direction, Mesh};

pub mod mesh;

use crate::{transform::Transform, utils::coord_to_index};

pub struct VoxelUniforms {
    pub view_projection: [[f32; 4]; 4],
    pub light_color: [f32; 3],
    pub light_position: [f32; 3],
}

pub type VoxelColor = [f32; 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voxel {
    Air,
    Stone,
    Grass,
    Dirt,
    Water,
    Sand,
    Snow,
}

impl Into<VoxelColor> for Voxel {
    fn into(self) -> VoxelColor {
        match self {
            Voxel::Air => [0.0, 0.0, 0.0, 0.0],
            Voxel::Stone => [0.69, 0.72, 0.72, 1.0],
            Voxel::Grass => [0.23, 0.82, 0.24, 1.0],
            Voxel::Dirt => [0.63, 0.45, 0.29, 1.0],
            Voxel::Water => [0.0, 0.62, 1.0, 0.8],
            Voxel::Sand => [0.93, 0.89, 0.55, 1.0],
            Voxel::Snow => [0.94, 0.98, 0.98, 1.0],
        }
    }
}

impl Voxel {
    pub fn is_air(self) -> bool {
        self == Voxel::Air
    }

    pub fn is_liquid(self) -> bool {
        self == Voxel::Water
    }

    pub fn is_solid(self) -> bool {
        !self.is_air() && !self.is_liquid()
    }
}

/// Represents a chunk of the world.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// The grid position of the chunk.
    pub grid_position: glam::IVec3,
    /// The size of the chunk.
    size: glam::UVec3,
    /// The transform of the chunk.
    transform: Transform,
    /// The voxels of the chunk.
    voxels: Vec<Voxel>,
}

impl Chunk {
    /// Creates a new empty chunk with the given grid position and size.
    pub fn new(grid_position: glam::IVec3, size: glam::UVec3) -> Self {
        let transform_position = grid_position * size.as_ivec3();

        Self {
            grid_position,
            size,
            transform: Transform {
                position: transform_position.as_vec3(),
                rotation: glam::Quat::IDENTITY,
                scale: glam::Vec3::ONE,
            },
            voxels: vec![Voxel::Air; size.x as usize * size.y as usize * size.z as usize],
        }
    }

    /// Returns the transformation of the chunk.
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// Returns a reference to the voxel at the given position.
    pub fn get_voxel(&self, position: glam::UVec3) -> Option<&Voxel> {
        if position.x >= self.size.x || position.y >= self.size.y || position.z >= self.size.z {
            return None;
        }

        let index = coord_to_index(position, self.size);
        self.voxels.get(index)
    }

    /// Sets the voxel at the given position.
    pub fn set_voxel(&mut self, position: glam::UVec3, voxel: Voxel) {
        let index = coord_to_index(position, self.size);
        if self.voxels.get(index).is_some() {
            self.voxels[index] = voxel;
        }
    }

    /// Returns whether the chunk entirely consists of air voxels.
    pub fn is_empty(&self) -> bool {
        self.voxels.iter().all(|voxel| voxel.is_air())
    }
}

impl Chunk {
    /// Creates a new mesh for the chunk.
    ///
    /// Returns a tuple of two optional meshes. The first mesh is the solid mesh and the second mesh is the transparent mesh.
    pub fn mesh(
        &self,
        chunk_neighbours: &HashMap<glam::IVec3, Chunk>,
    ) -> (Option<Mesh>, Option<Mesh>) {
        let mesh = {
            let mesh = self.greedy_mesh(
                chunk_neighbours,
                |voxel| voxel.is_solid(),
                |voxel| !voxel.is_solid(),
            );
            if mesh.is_empty() {
                None
            } else {
                Some(mesh)
            }
        };

        let transparent_mesh = {
            let mesh = self.greedy_mesh(
                chunk_neighbours,
                |voxel| voxel.is_liquid(),
                |voxel| voxel.is_air(),
            );
            if mesh.is_empty() {
                None
            } else {
                Some(mesh)
            }
        };

        (mesh, transparent_mesh)
    }

    fn greedy_mesh<V, N>(
        &self,
        chunk_neighbours: &HashMap<glam::IVec3, Chunk>,
        voxel_to_mesh: V,
        neighbour_condition: N,
    ) -> Mesh
    where
        V: Fn(Voxel) -> bool,
        N: Fn(Voxel) -> bool,
    {
        let mut mesh = Mesh::new();

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            for direction in [Direction::Positive, Direction::Negative] {
                let mut visited = HashSet::new();

                let plane_dimensions = match axis {
                    Axis::X => glam::uvec2(self.size.y, self.size.z),
                    Axis::Y => glam::uvec2(self.size.x, self.size.z),
                    Axis::Z => glam::uvec2(self.size.x, self.size.y),
                };

                for x in 0..self.size.x {
                    for y in 0..self.size.y {
                        for z in 0..self.size.z {
                            let position = glam::uvec3(x, y, z);
                            let voxel = self.get_voxel(position);
                            if visited.contains(&position) || voxel.is_none() {
                                continue;
                            } else if let Some(voxel) = voxel {
                                if !voxel_to_mesh(*voxel) {
                                    continue;
                                }
                            }

                            let neighbour_position =
                                (position.as_vec3() + axis.get_normal(direction)).as_ivec3();
                            if neighbour_position.x < 0
                                || neighbour_position.x >= self.size.x as i32
                                || neighbour_position.y < 0
                                || neighbour_position.y >= self.size.y as i32
                                || neighbour_position.z < 0
                                || neighbour_position.z >= self.size.z as i32
                            {
                                if !self.voxel_has_neigbour(
                                    chunk_neighbours,
                                    position,
                                    axis,
                                    direction,
                                    &neighbour_condition,
                                ) {
                                    continue;
                                }
                            } else {
                                let neighbour_voxel = self.get_voxel(neighbour_position.as_uvec3());
                                if let Some(neighbour_voxel) = neighbour_voxel {
                                    if !neighbour_condition(*neighbour_voxel) {
                                        continue;
                                    }
                                }
                            }

                            let plane = match axis {
                                Axis::X => glam::uvec2(y, z),
                                Axis::Y => glam::uvec2(x, z),
                                Axis::Z => glam::uvec2(x, y),
                            };
                            let mut size = glam::uvec2(1, 1);
                            while plane.x + size.x < plane_dimensions.x {
                                let next_position = match axis {
                                    Axis::X => glam::uvec3(x, y + size.x, z),
                                    Axis::Y => glam::uvec3(x + size.x, y, z),
                                    Axis::Z => glam::uvec3(x + size.x, y, z),
                                };

                                let next_voxel = self.get_voxel(next_position);
                                if visited.contains(&next_position)
                                    || next_voxel.is_none()
                                    || next_voxel != voxel
                                {
                                    break;
                                }

                                size.x += 1;
                            }

                            'outer: while plane.y + size.y < plane_dimensions.y {
                                for w in 0..size.x {
                                    let next_position = match axis {
                                        Axis::X => glam::uvec3(x, y + w, z + size.y),
                                        Axis::Y => glam::uvec3(x + w, y, z + size.y),
                                        Axis::Z => glam::uvec3(x + w, y + size.y, z),
                                    };

                                    let next_voxel = self.get_voxel(next_position);
                                    if visited.contains(&next_position)
                                        || next_voxel.is_none()
                                        || next_voxel != voxel
                                    {
                                        break 'outer;
                                    }
                                }
                                size.y += 1;
                            }

                            mesh.add_face(
                                position.as_vec3()
                                    + match direction {
                                        Direction::Positive => axis.get_normal(direction),
                                        Direction::Negative => glam::Vec3::ZERO,
                                    },
                                size.as_vec2(),
                                axis,
                                direction,
                                Into::<VoxelColor>::into(*voxel.unwrap()),
                            );

                            for w in 0..size.x {
                                for h in 0..size.y {
                                    let visited_position = match axis {
                                        Axis::X => glam::uvec3(x, y + w, z + h),
                                        Axis::Y => glam::uvec3(x + w, y, z + h),
                                        Axis::Z => glam::uvec3(x + w, y + h, z),
                                    };
                                    visited.insert(visited_position);
                                }
                            }
                        }
                    }
                }
            }
        }

        mesh
    }

    fn voxel_has_neigbour(
        &self,
        chunk_neighbours: &HashMap<glam::IVec3, Chunk>,
        voxel_position: glam::UVec3,
        axis: Axis,
        direction: Direction,
        condition: impl Fn(Voxel) -> bool,
    ) -> bool {
        let neighbour_chunk_position = self.grid_position + axis.get_normal(direction).as_ivec3();

        if let Some(neighbour_chunk) = chunk_neighbours.get(&neighbour_chunk_position) {
            let neighbour_position = match (axis, direction) {
                (Axis::X, Direction::Positive) => {
                    glam::uvec3(0, voxel_position.y, voxel_position.z)
                }
                (Axis::X, Direction::Negative) => {
                    glam::uvec3(self.size.x - 1, voxel_position.y, voxel_position.z)
                }
                (Axis::Y, Direction::Positive) => {
                    glam::uvec3(voxel_position.x, 0, voxel_position.z)
                }
                (Axis::Y, Direction::Negative) => {
                    glam::uvec3(voxel_position.x, self.size.y - 1, voxel_position.z)
                }
                (Axis::Z, Direction::Positive) => {
                    glam::uvec3(voxel_position.x, voxel_position.y, 0)
                }
                (Axis::Z, Direction::Negative) => {
                    glam::uvec3(voxel_position.x, voxel_position.y, self.size.z - 1)
                }
            };

            if let Some(neighbour_voxel) = neighbour_chunk.get_voxel(neighbour_position) {
                if condition(*neighbour_voxel) {
                    return true;
                }
            }
        }

        false
    }
}
