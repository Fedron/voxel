pub fn coord_to_index(position: glam::UVec3, size: glam::UVec3) -> usize {
    position.x as usize
        + position.y as usize * size.x as usize
        + position.z as usize * size.x as usize * size.y as usize
}

pub fn index_to_coord(index: usize, size: glam::UVec3) -> glam::UVec3 {
    glam::uvec3(
        index as u32 % size.x,
        (index as u32 / size.x) % size.y,
        index as u32 / (size.x * size.y),
    )
}
