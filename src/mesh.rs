use std::sync::Arc;

use vulkano::{
    acceleration_structure::{
        AccelerationStructure, AccelerationStructureBuildGeometryInfo,
        AccelerationStructureBuildRangeInfo, AccelerationStructureBuildSizesInfo,
        AccelerationStructureBuildType, AccelerationStructureCreateInfo,
        AccelerationStructureGeometries, AccelerationStructureGeometryInstancesData,
        AccelerationStructureGeometryInstancesDataType, AccelerationStructureGeometryTrianglesData,
        AccelerationStructureInstance, AccelerationStructureType, BuildAccelerationStructureFlags,
        BuildAccelerationStructureMode, GeometryFlags, GeometryInstanceFlags,
    },
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, IndexBuffer, Subbuffer},
    command_buffer::{
        allocator::CommandBufferAllocator, CommandBufferBeginInfo, CommandBufferLevel,
        CommandBufferUsage, RecordingCommandBuffer,
    },
    device::Queue,
    memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter},
    pipeline::graphics::vertex_input::Vertex as VertexTrait,
    sync::GpuFuture,
    DeviceSize, Packed24_8,
};

#[derive(Debug, Clone, Copy, BufferContents, VertexTrait)]
#[repr(C)]
pub struct Vertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn as_blas(
        &self,
        memory_allocator: Arc<dyn MemoryAllocator>,
        command_buffer_allocator: Arc<dyn CommandBufferAllocator>,
        queue: Arc<Queue>,
    ) -> Arc<AccelerationStructure> {
        let vertex_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY
                    | BufferUsage::SHADER_DEVICE_ADDRESS,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            self.vertices.clone(),
        )
        .unwrap();

        let index_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY
                    | BufferUsage::SHADER_DEVICE_ADDRESS,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            self.indices.clone(),
        )
        .unwrap();

        let max_vertex = vertex_buffer.len() as u32;
        let primitive_count = max_vertex / 3;
        let triangles = AccelerationStructureGeometryTrianglesData {
            flags: GeometryFlags::OPAQUE,
            vertex_data: Some(vertex_buffer.into_bytes()),
            vertex_stride: Vertex::per_vertex().stride,
            max_vertex,
            index_data: Some(IndexBuffer::U32(index_buffer)),
            transform_data: None,
            ..AccelerationStructureGeometryTrianglesData::new(
                Vertex::per_vertex().members.get("position").unwrap().format,
            )
        };

        let build_range_info = AccelerationStructureBuildRangeInfo {
            primitive_count,
            primitive_offset: 0,
            first_vertex: 0,
            transform_offset: 0,
        };

        let geometries = AccelerationStructureGeometries::Triangles(vec![triangles]);
        let build_info = AccelerationStructureBuildGeometryInfo {
            flags: BuildAccelerationStructureFlags::PREFER_FAST_TRACE,
            mode: BuildAccelerationStructureMode::Build,
            ..AccelerationStructureBuildGeometryInfo::new(geometries)
        };

        build_acceleration_structure(
            AccelerationStructureType::BottomLevel,
            memory_allocator,
            command_buffer_allocator,
            queue,
            build_info,
            build_range_info,
            primitive_count,
        )
    }
}

pub fn create_tlas(
    memory_allocator: Arc<dyn MemoryAllocator>,
    command_buffer_allocator: Arc<dyn CommandBufferAllocator>,
    queue: Arc<Queue>,
    blas: &[&AccelerationStructure],
) -> Arc<AccelerationStructure> {
    let instances = blas
        .iter()
        .map(|&blas| AccelerationStructureInstance {
            instance_shader_binding_table_record_offset_and_flags: Packed24_8::new(
                0,
                GeometryInstanceFlags::TRIANGLE_FACING_CULL_DISABLE.into(),
            ),
            acceleration_structure_reference: blas.device_address().get(),
            ..Default::default()
        })
        .collect::<Vec<_>>();

    let values = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY
                | BufferUsage::SHADER_DEVICE_ADDRESS,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        instances,
    )
    .unwrap();

    let geometries =
        AccelerationStructureGeometries::Instances(AccelerationStructureGeometryInstancesData {
            flags: GeometryFlags::OPAQUE,
            ..AccelerationStructureGeometryInstancesData::new(
                AccelerationStructureGeometryInstancesDataType::Values(Some(values)),
            )
        });

    let build_info = AccelerationStructureBuildGeometryInfo {
        flags: BuildAccelerationStructureFlags::PREFER_FAST_TRACE,
        mode: BuildAccelerationStructureMode::Build,
        ..AccelerationStructureBuildGeometryInfo::new(geometries)
    };

    let build_range_info = AccelerationStructureBuildRangeInfo {
        primitive_count: blas.len() as _,
        primitive_offset: 0,
        first_vertex: 0,
        transform_offset: 0,
    };

    build_acceleration_structure(
        AccelerationStructureType::TopLevel,
        memory_allocator,
        command_buffer_allocator,
        queue,
        build_info,
        build_range_info,
        blas.len() as u32,
    )
}

fn create_acceleration_structure(
    ty: AccelerationStructureType,
    memory_allocator: Arc<dyn MemoryAllocator>,
    size: DeviceSize,
) -> Arc<AccelerationStructure> {
    let buffer = Buffer::new_slice::<u8>(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::ACCELERATION_STRUCTURE_STORAGE,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
        size,
    )
    .unwrap();

    unsafe {
        AccelerationStructure::new(
            memory_allocator.device().clone(),
            AccelerationStructureCreateInfo {
                ty,
                ..AccelerationStructureCreateInfo::new(buffer)
            },
        )
        .unwrap()
    }
}

fn create_scratch_buffer(
    memory_allocator: Arc<dyn MemoryAllocator>,
    size: DeviceSize,
) -> Subbuffer<[u8]> {
    Buffer::new_slice::<u8>(
        memory_allocator,
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER | BufferUsage::SHADER_DEVICE_ADDRESS,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
        size,
    )
    .unwrap()
}

fn build_acceleration_structure(
    ty: AccelerationStructureType,
    memory_allocator: Arc<dyn MemoryAllocator>,
    command_buffer_allocator: Arc<dyn CommandBufferAllocator>,
    queue: Arc<Queue>,
    mut build_info: AccelerationStructureBuildGeometryInfo,
    build_range_info: AccelerationStructureBuildRangeInfo,
    max_primitive_count: u32,
) -> Arc<AccelerationStructure> {
    let device = memory_allocator.device();

    let AccelerationStructureBuildSizesInfo {
        acceleration_structure_size,
        build_scratch_size,
        ..
    } = device
        .acceleration_structure_build_sizes(
            AccelerationStructureBuildType::Device,
            &build_info,
            &[max_primitive_count],
        )
        .unwrap();

    let acceleration_structure =
        create_acceleration_structure(ty, memory_allocator.clone(), acceleration_structure_size);
    let scratch_buffer = create_scratch_buffer(memory_allocator.clone(), build_scratch_size);

    build_info.dst_acceleration_structure = Some(acceleration_structure.clone());
    build_info.scratch_data = Some(scratch_buffer);

    let mut builder = RecordingCommandBuffer::new(
        command_buffer_allocator,
        queue.queue_family_index(),
        CommandBufferLevel::Primary,
        CommandBufferBeginInfo {
            usage: CommandBufferUsage::OneTimeSubmit,
            ..Default::default()
        },
    )
    .unwrap();

    unsafe {
        builder
            .build_acceleration_structure(build_info, [build_range_info].into_iter().collect())
            .unwrap();
    }

    let command_buffer = builder.end().unwrap();
    command_buffer
        .execute(queue)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    acceleration_structure
}
