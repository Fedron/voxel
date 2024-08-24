use bon::bon;
use glam::FloatExt;
use noise::{core::perlin::perlin_2d, permutationtable::PermutationTable};

use crate::chunk::{Chunk, Voxel};

pub struct WorldGenerator {
    chunk_size: glam::UVec3,
    max_world_height: u32,

    permutation_table: PermutationTable,
}

#[bon]
impl WorldGenerator {
    #[builder]
    pub fn new(seed: u32, chunk_size: glam::UVec3, max_world_height: u32) -> Self {
        let permutation_table = PermutationTable::new(seed);

        Self {
            chunk_size,
            max_world_height,
            permutation_table,
        }
    }

    pub fn generate_chunk(&self, world_position: glam::IVec3) -> Chunk {
        let mut chunk = Chunk::new(glam::ivec3(
            world_position.x * self.chunk_size.x as i32,
            world_position.y * self.chunk_size.y as i32,
            world_position.z * self.chunk_size.z as i32,
        ));

        for x in 0..self.chunk_size.x {
            for z in 0..self.chunk_size.z {
                let height = perlin_2d(
                    (
                        ((world_position.x * self.chunk_size.x as i32) + x as i32) as f64 / 128.0,
                        ((world_position.z * self.chunk_size.z as i32) + z as i32) as f64 / 128.0,
                    )
                        .into(),
                    &self.permutation_table,
                )
                .remap(-1.0, 1.0, 0.0, self.max_world_height as f64)
                .floor() as i32;

                for y in 0..self.chunk_size.y as i32 {
                    if (world_position.y * self.chunk_size.y as i32) + y < height {
                        chunk.set_voxel(glam::UVec3::new(x, y as u32, z), Voxel::Stone);
                    }
                }
            }
        }

        chunk
    }
}
