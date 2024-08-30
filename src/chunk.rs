use std::collections::{HashMap, HashSet};

use mesh::{Axis, Direction, Mesh};

pub mod mesh;

use crate::{
    transform::Transform,
    utils::{coord_to_index, index_to_coord},
};

pub struct VoxelUniforms {
    pub view_projection: [[f32; 4]; 4],
    pub light_color: [f32; 3],
    pub light_position: [f32; 3],
}

#[derive(Copy, Clone)]
pub struct VoxelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: VoxelColor,
}
implement_vertex!(VoxelVertex, position, normal, color);

pub type VoxelColor = [f32; 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voxel {
    Air,
    Stone,
    Grass,
    Dirt,
    Water,
    Sand,
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

pub const CHUNK_SIZE: glam::UVec3 = glam::uvec3(16, 16, 16);

pub struct Chunk {
    grid_position: glam::UVec3,
    size: glam::UVec3,
    transform: Transform,
    voxels: Vec<Voxel>,
}

impl Chunk {
    pub fn new(grid_position: glam::UVec3, size: glam::UVec3) -> Self {
        let transform_position = grid_position * size;

        Self {
            grid_position,
            size,
            transform: Transform {
                position: transform_position.as_vec3(),
                rotation: glam::Quat::IDENTITY,
                scale: glam::Vec3::ONE,
            },
            voxels: vec![
                Voxel::Air;
                CHUNK_SIZE.x as usize * CHUNK_SIZE.y as usize * CHUNK_SIZE.z as usize
            ],
        }
    }

    pub fn get_voxel(&self, position: glam::UVec3) -> Option<&Voxel> {
        if position.x >= CHUNK_SIZE.x || position.y >= CHUNK_SIZE.y || position.z >= CHUNK_SIZE.z {
            return None;
        }

        let index = coord_to_index(position, CHUNK_SIZE);
        self.voxels.get(index)
    }

    pub fn set_voxel(&mut self, position: glam::UVec3, voxel: Voxel) {
        let index = coord_to_index(position, CHUNK_SIZE);
        if self.voxels.get(index).is_some() {
            self.voxels[index] = voxel;
        }
    }

    pub fn iter(&self) -> ChunkIterator {
        ChunkIterator {
            chunk: self,
            current_index: 0,
        }
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }
}

pub struct ChunkIterator<'a> {
    chunk: &'a Chunk,
    current_index: usize,
}

impl<'a> Iterator for ChunkIterator<'a> {
    type Item = (&'a Voxel, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index < self.chunk.voxels.len() {
            let voxel = &self.chunk.voxels[self.current_index];
            let index = self.current_index;
            self.current_index += 1;
            Some((voxel, index))
        } else {
            None
        }
    }
}

pub struct ChunkMesher {}

impl ChunkMesher {
    pub fn mesh(chunk: &Chunk, neighbours: HashMap<glam::UVec3, &Chunk>) -> Mesh {
        let mut mesh = Mesh::new();

        // TODO: If this meshing is kept, transparency will need to be reimplemented
        for (voxel, index) in chunk.iter() {
            match voxel {
                Voxel::Stone | Voxel::Grass | Voxel::Dirt | Voxel::Sand | Voxel::Water => {
                    let neighbours = Self::get_neighbouring_voxels(
                        chunk,
                        &neighbours,
                        index_to_coord(index, CHUNK_SIZE).as_ivec3(),
                        |v| !v.is_solid(),
                    );

                    for i in 0..6 {
                        if (neighbours >> i) & 1 == 1 {
                            let position = index_to_coord(index, CHUNK_SIZE);
                            let base_position =
                                glam::vec3(position.x as f32, position.y as f32, position.z as f32);

                            match i {
                                0 => mesh.add_face(
                                    base_position,
                                    glam::vec2(1.0, 1.0),
                                    Axis::Z,
                                    Direction::Negative,
                                    Into::<VoxelColor>::into(*voxel),
                                ),
                                1 => mesh.add_face(
                                    base_position + glam::vec3(0.0, 0.0, 1.0),
                                    glam::vec2(1.0, 1.0),
                                    Axis::Z,
                                    Direction::Positive,
                                    Into::<VoxelColor>::into(*voxel),
                                ),
                                2 => mesh.add_face(
                                    base_position + glam::vec3(0.0, 1.0, 0.0),
                                    glam::vec2(1.0, 1.0),
                                    Axis::Y,
                                    Direction::Positive,
                                    Into::<VoxelColor>::into(*voxel),
                                ),
                                3 => mesh.add_face(
                                    base_position,
                                    glam::vec2(1.0, 1.0),
                                    Axis::Y,
                                    Direction::Negative,
                                    Into::<VoxelColor>::into(*voxel),
                                ),
                                4 => mesh.add_face(
                                    base_position,
                                    glam::vec2(1.0, 1.0),
                                    Axis::X,
                                    Direction::Negative,
                                    Into::<VoxelColor>::into(*voxel),
                                ),
                                5 => mesh.add_face(
                                    base_position + glam::vec3(1.0, 0.0, 0.0),
                                    glam::vec2(1.0, 1.0),
                                    Axis::X,
                                    Direction::Positive,
                                    Into::<VoxelColor>::into(*voxel),
                                ),
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        mesh
    }

    fn get_neighbouring_voxels<C>(
        chunk: &Chunk,
        chunk_neighbours: &HashMap<glam::UVec3, &Chunk>,
        voxel_position: glam::IVec3,
        condition: C,
    ) -> u8
    where
        C: Fn(Voxel) -> bool,
    {
        let mut mask = 0;
        for i in 0..6 {
            let face_direction = match i {
                0 => Axis::Z.get_normal(Direction::Negative),
                1 => Axis::Z.get_normal(Direction::Positive),
                2 => Axis::Y.get_normal(Direction::Positive),
                3 => Axis::Y.get_normal(Direction::Negative),
                4 => Axis::X.get_normal(Direction::Negative),
                5 => Axis::X.get_normal(Direction::Positive),
                _ => unreachable!(),
            }
            .as_ivec3();

            // If neighbour is within the same chunk, check voxel in the chunk
            let neighbour_position = voxel_position + face_direction;
            if neighbour_position.x >= 0
                && neighbour_position.x < chunk.size.x as i32
                && neighbour_position.y >= 0
                && neighbour_position.y < chunk.size.y as i32
                && neighbour_position.z >= 0
                && neighbour_position.z < chunk.size.z as i32
            {
                if let Some(neighbour) = chunk.get_voxel(neighbour_position.as_uvec3()) {
                    if condition(*neighbour) {
                        mask |= 1 << i;
                    }
                }
            }
            // If neighbour is out of bounds for this chunk, try checking the corresponding neighbouring chunk
            else {
                let neighbour_chunk_position = chunk.grid_position.as_ivec3() + face_direction;
                let neighbour_chunk_position: Result<glam::UVec3, _> =
                    neighbour_chunk_position.try_into();

                match neighbour_chunk_position {
                    Ok(neighbour) => {
                        if let Some(neighbour_chunk) = chunk_neighbours.get(&neighbour) {
                            let neighbour_position = match face_direction {
                                glam::IVec3::X => {
                                    glam::uvec3(0, voxel_position.y as u32, voxel_position.z as u32)
                                }
                                glam::IVec3::Y => {
                                    glam::uvec3(voxel_position.x as u32, 0, voxel_position.z as u32)
                                }
                                glam::IVec3::Z => {
                                    glam::uvec3(voxel_position.x as u32, voxel_position.y as u32, 0)
                                }
                                glam::IVec3::NEG_X => glam::uvec3(
                                    CHUNK_SIZE.x - 1,
                                    voxel_position.y as u32,
                                    voxel_position.z as u32,
                                ),
                                glam::IVec3::NEG_Y => glam::uvec3(
                                    voxel_position.x as u32,
                                    CHUNK_SIZE.y - 1,
                                    voxel_position.z as u32,
                                ),
                                glam::IVec3::NEG_Z => glam::uvec3(
                                    voxel_position.x as u32,
                                    voxel_position.y as u32,
                                    CHUNK_SIZE.z - 1,
                                ),
                                _ => unreachable!(),
                            };

                            if let Some(neighbour) = neighbour_chunk.get_voxel(neighbour_position) {
                                if condition(*neighbour) {
                                    mask |= 1 << i;
                                }
                            }
                        } else {
                            mask |= 1 << i;
                        }
                    }
                    Err(_) => {
                        mask |= 1 << i;
                    }
                }
            }
        }
        mask
    }

    pub fn greedy_mesh(chunk: &Chunk, chunk_neighbours: &HashMap<glam::UVec3, &Chunk>) -> Mesh {
        let mut mesh = Mesh::new();

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            for direction in [Direction::Positive, Direction::Negative] {
                let mut visited = HashSet::new();

                let plane_dimensions = match axis {
                    Axis::X => glam::uvec2(chunk.size.y, chunk.size.z),
                    Axis::Y => glam::uvec2(chunk.size.x, chunk.size.z),
                    Axis::Z => glam::uvec2(chunk.size.x, chunk.size.y),
                };

                for x in 0..chunk.size.x {
                    for y in 0..chunk.size.y {
                        for z in 0..chunk.size.z {
                            let position = glam::uvec3(x, y, z);
                            let voxel = chunk.get_voxel(position);
                            if visited.contains(&position) || voxel.is_none() {
                                continue;
                            } else if let Some(voxel) = voxel {
                                if voxel.is_air() {
                                    continue;
                                }
                            }

                            let neighbour_position =
                                (position.as_vec3() + axis.get_normal(direction)).as_ivec3();
                            if neighbour_position.x < 0
                                || neighbour_position.y < 0
                                || neighbour_position.z < 0
                            {
                                if Self::has_neigbour(
                                    chunk,
                                    chunk_neighbours,
                                    position,
                                    axis,
                                    direction,
                                    |v| !v.is_air(),
                                ) {
                                    continue;
                                }
                            } else {
                                let neighbour_voxel =
                                    chunk.get_voxel(neighbour_position.as_uvec3());

                                match neighbour_voxel {
                                    Some(neighbour) => {
                                        if !neighbour.is_air() {
                                            continue;
                                        }
                                    }
                                    None => {
                                        if Self::has_neigbour(
                                            chunk,
                                            chunk_neighbours,
                                            position,
                                            axis,
                                            direction,
                                            |v| !v.is_air(),
                                        ) {
                                            continue;
                                        }
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

                                let next_voxel = chunk.get_voxel(next_position);
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

                                    let next_voxel = chunk.get_voxel(next_position);
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

    fn has_neigbour(
        chunk: &Chunk,
        chunk_neighbours: &HashMap<glam::UVec3, &Chunk>,
        voxel_position: glam::UVec3,
        axis: Axis,
        direction: Direction,
        condition: impl Fn(Voxel) -> bool,
    ) -> bool {
        let neighbour_chunk_position =
            chunk.grid_position.as_ivec3() + axis.get_normal(direction).as_ivec3();
        let neighbour_chunk_position: Result<glam::UVec3, _> = neighbour_chunk_position.try_into();

        if let Ok(neighbour_chunk_position) = neighbour_chunk_position {
            if let Some(neighbour_chunk) = chunk_neighbours.get(&neighbour_chunk_position) {
                let neighbour_position = match (axis, direction) {
                    (Axis::X, Direction::Positive) => {
                        glam::uvec3(0, voxel_position.y, voxel_position.z)
                    }
                    (Axis::X, Direction::Negative) => {
                        glam::uvec3(chunk.size.x - 1, voxel_position.y, voxel_position.z)
                    }
                    (Axis::Y, Direction::Positive) => {
                        glam::uvec3(voxel_position.x, 0, voxel_position.z)
                    }
                    (Axis::Y, Direction::Negative) => {
                        glam::uvec3(voxel_position.x, chunk.size.y - 1, voxel_position.z)
                    }
                    (Axis::Z, Direction::Positive) => {
                        glam::uvec3(voxel_position.x, voxel_position.y, 0)
                    }
                    (Axis::Z, Direction::Negative) => {
                        glam::uvec3(voxel_position.x, voxel_position.y, chunk.size.z - 1)
                    }
                };

                if let Some(neighbour_voxel) = neighbour_chunk.get_voxel(neighbour_position) {
                    if condition(*neighbour_voxel) {
                        return true;
                    }
                }
            }
        }

        false
    }
}
