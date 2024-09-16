use anyhow::Result;

use app::{App, AppConfig, ImageAndView};
use vulkan::{
    ash::vk::{self, Packed24_8},
    utils::create_gpu_only_buffer_from_data,
    AccelerationStructure, Buffer, Context, DescriptorPool, DescriptorSet, DescriptorSetLayout,
    PipelineLayout, RayTracingPipeline, RayTracingPipelineCreateInfo, RayTracingShaderCreateInfo,
    RayTracingShaderGroup, ShaderBindingTable, WriteDescriptorSet, WriteDescriptorSetKind,
};

fn main() -> Result<()> {
    app::run::<VoxelApp>(
        "Voxel",
        1920,
        1080,
        AppConfig {
            enable_raytracing: true,
            ..Default::default()
        },
    )
}

struct VoxelApp {
    _blas: Blas,
    _tlas: Tlas,
    pipeline: Pipeline,
    shader_binding_table: ShaderBindingTable,
    descriptor_sets: DescriptorSets,
}

impl App for VoxelApp {
    type Gui = ();

    fn new(base: &mut app::BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let blas = Blas::new(context)?;
        let tlas = Tlas::new(context, &[&blas])?;
        let pipeline = Pipeline::new(context)?;
        let shader_binding_table = context.create_shader_binding_table(&pipeline.inner)?;
        let descriptor_sets = DescriptorSets::new(context, &pipeline, &tlas, &base.storage_images)?;

        Ok(Self {
            _blas: blas,
            _tlas: tlas,
            pipeline,
            shader_binding_table,
            descriptor_sets,
        })
    }

    fn update(
        &mut self,
        _base: &mut app::BaseApp<Self>,
        _image_index: usize,
        _delta_time: std::time::Duration,
    ) -> Result<()> {
        Ok(())
    }

    fn record_raytracing_commands(
        &self,
        base: &app::BaseApp<Self>,
        buffer: &vulkan::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        let static_set = &self.descriptor_sets.static_set;
        let dynamic_set = &self.descriptor_sets.dynamic_sets[image_index];

        buffer.bind_rt_pipeline(&self.pipeline.inner);
        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::RAY_TRACING_KHR,
            &self.pipeline.layout,
            0,
            &[static_set, dynamic_set],
        );
        buffer.trace_rays(
            &self.shader_binding_table,
            base.swapchain.extent.width,
            base.swapchain.extent.height,
        );

        Ok(())
    }

    fn record_raster_commands(&self, base: &app::BaseApp<Self>, image_index: usize) -> Result<()> {
        let _ = base;
        let _ = image_index;

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, base: &app::BaseApp<Self>) -> Result<()> {
        base.storage_images
            .iter()
            .enumerate()
            .for_each(|(index, image)| {
                let set = &self.descriptor_sets.dynamic_sets[index];
                set.update(&[WriteDescriptorSet {
                    binding: 1,
                    kind: WriteDescriptorSetKind::StorageImage {
                        view: &image.view,
                        layout: vk::ImageLayout::GENERAL,
                    },
                }])
            });

        Ok(())
    }
}

struct Blas {
    inner: AccelerationStructure,
    _vertex_buffer: Buffer,
    _index_buffer: Buffer,
}

impl Blas {
    fn new(context: &mut Context) -> Result<Self> {
        #[derive(Debug, Clone, Copy)]
        #[allow(dead_code)]
        struct Vertex {
            pos: [f32; 2],
        }

        const VERTICES: [Vertex; 3] = [
            Vertex { pos: [-1.0, 1.0] },
            Vertex { pos: [1.0, 1.0] },
            Vertex { pos: [0.0, -1.0] },
        ];
        let vertex_buffer = create_gpu_only_buffer_from_data(
            context,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            &VERTICES,
        )?;
        let vertex_buffer_addr = vertex_buffer.get_device_address();

        const INDICES: [u16; 3] = [0, 1, 2];
        let index_buffer = create_gpu_only_buffer_from_data(
            context,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            &INDICES,
        )?;
        let index_buffer_addr = index_buffer.get_device_address();

        let geometry_triangles = vk::AccelerationStructureGeometryTrianglesDataKHR::default()
            .vertex_format(vk::Format::R32G32_SFLOAT)
            .vertex_data(vk::DeviceOrHostAddressConstKHR {
                device_address: vertex_buffer_addr,
            })
            .vertex_stride(size_of::<Vertex>() as _)
            .index_type(vk::IndexType::UINT16)
            .index_data(vk::DeviceOrHostAddressConstKHR {
                device_address: index_buffer_addr,
            })
            .max_vertex(INDICES.len() as _);

        let geometry = vk::AccelerationStructureGeometryKHR::default()
            .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
            .flags(vk::GeometryFlagsKHR::OPAQUE)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                triangles: geometry_triangles,
            });

        let build_range_info = vk::AccelerationStructureBuildRangeInfoKHR::default()
            .first_vertex(0)
            .primitive_count((INDICES.len() / 3) as _)
            .primitive_offset(0)
            .transform_offset(0);

        let inner = context.create_bottom_level_acceleration_structure(
            &[geometry],
            &[build_range_info],
            &[(INDICES.len() / 3) as _],
        )?;

        Ok(Blas {
            inner,
            _vertex_buffer: vertex_buffer,
            _index_buffer: index_buffer,
        })
    }
}

struct Tlas {
    inner: AccelerationStructure,
    _instances_buffer: Buffer,
}

impl Tlas {
    fn new(context: &mut Context, blas: &[&Blas]) -> Result<Self> {
        #[rustfmt::skip]
        let transform_matrix = vk::TransformMatrixKHR { matrix: [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
        ]};

        let mut instances = Vec::with_capacity(blas.len());
        for &blas in blas {
            let instance: vk::AccelerationStructureInstanceKHR =
                vk::AccelerationStructureInstanceKHR {
                    transform: transform_matrix.clone(),
                    instance_custom_index_and_mask: Packed24_8::new(0, 0xFF),
                    instance_shader_binding_table_record_offset_and_flags: Packed24_8::new(
                        0,
                        vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE
                            .as_raw()
                            .try_into()?,
                    ),
                    acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                        device_handle: blas.inner.address,
                    },
                };

            instances.push(instance);
        }

        let instances_buffer = create_gpu_only_buffer_from_data(
            context,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            &instances,
        )?;
        let instances_buffer_addr = instances_buffer.get_device_address();

        let geometry = vk::AccelerationStructureGeometryKHR::default()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .flags(vk::GeometryFlagsKHR::OPAQUE)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: vk::AccelerationStructureGeometryInstancesDataKHR::default()
                    .array_of_pointers(false)
                    .data(vk::DeviceOrHostAddressConstKHR {
                        device_address: instances_buffer_addr,
                    }),
            });

        let build_range_info = vk::AccelerationStructureBuildRangeInfoKHR::default()
            .first_vertex(0)
            .primitive_count(blas.len() as _)
            .primitive_offset(0)
            .transform_offset(0);

        let inner = context.create_top_level_acceleration_structure(
            &[geometry],
            &[build_range_info],
            &[blas.len() as _],
        )?;

        Ok(Self {
            inner,
            _instances_buffer: instances_buffer,
        })
    }
}

struct Pipeline {
    inner: RayTracingPipeline,
    layout: PipelineLayout,
    static_dsl: DescriptorSetLayout,
    dynamic_dsl: DescriptorSetLayout,
}

impl Pipeline {
    fn new(context: &Context) -> Result<Self> {
        let static_layout_bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR)];

        let dynamic_layout_bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)];

        let static_dsl = context.create_descriptor_set_layout(&static_layout_bindings)?;
        let dynamic_dsl = context.create_descriptor_set_layout(&dynamic_layout_bindings)?;
        let dsls = [&static_dsl, &dynamic_dsl];

        let layout = context.create_pipeline_layout(&dsls)?;

        let shaders_create_info = [
            RayTracingShaderCreateInfo {
                source: &include_bytes!("../shaders/raygen.rgen.spv")[..],
                stage: vk::ShaderStageFlags::RAYGEN_KHR,
                group: RayTracingShaderGroup::RayGen,
            },
            RayTracingShaderCreateInfo {
                source: &include_bytes!("../shaders/miss.rmiss.spv")[..],
                stage: vk::ShaderStageFlags::MISS_KHR,
                group: RayTracingShaderGroup::Miss,
            },
            RayTracingShaderCreateInfo {
                source: &include_bytes!("../shaders/closesthit.rchit.spv")[..],
                stage: vk::ShaderStageFlags::CLOSEST_HIT_KHR,
                group: RayTracingShaderGroup::ClosestHit,
            },
        ];

        let inner = context.create_ray_tracing_pipeline(
            &layout,
            RayTracingPipelineCreateInfo {
                shaders: &shaders_create_info,
                max_ray_recursion_depth: 1,
            },
        )?;

        Ok(Self {
            inner,
            layout,
            static_dsl,
            dynamic_dsl,
        })
    }
}

struct DescriptorSets {
    _pool: DescriptorPool,
    static_set: DescriptorSet,
    dynamic_sets: Vec<DescriptorSet>,
}

impl DescriptorSets {
    fn new(
        context: &Context,
        pipeline: &Pipeline,
        tlas: &Tlas,
        storage_images: &[ImageAndView],
    ) -> Result<Self> {
        let set_count = storage_images.len() as u32;

        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .descriptor_count(1),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(set_count),
        ];
        let pool = context.create_descriptor_pool(set_count + 1, &pool_sizes)?;

        let static_set = pool.allocate_set(&pipeline.static_dsl)?;
        let dynamic_sets = pool.allocate_sets(&pipeline.dynamic_dsl, set_count)?;

        static_set.update(&[WriteDescriptorSet {
            binding: 0,
            kind: WriteDescriptorSetKind::AccelerationStructure {
                acceleration_structure: &tlas.inner,
            },
        }]);

        dynamic_sets.iter().enumerate().for_each(|(index, set)| {
            set.update(&[WriteDescriptorSet {
                binding: 1,
                kind: WriteDescriptorSetKind::StorageImage {
                    view: &storage_images[index].view,
                    layout: vk::ImageLayout::GENERAL,
                },
            }]);
        });

        Ok(Self {
            _pool: pool,
            static_set,
            dynamic_sets,
        })
    }
}
