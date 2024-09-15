use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};

use anyhow::Result;
use simplelog::TermLogger;
use vulkan::{
    ash::vk, gpu_allocator::MemoryLocation, AcquiredImage, CommandBuffer, CommandPool, Context,
    ContextBuilder, DeviceFeatures, Fence, Image, ImageBarrier, ImageView, Semaphore,
    SemaphoreSubmitInfo, Swapchain, TimestampQueryPool, VERSION_1_3,
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::camera::{Camera, CameraControls};

const IN_FLIGHT_FRAMES: u32 = 2;

#[derive(Debug, Default)]
pub struct AppConfig<'a, 'b> {
    pub enable_raytracing: bool,
    pub required_instance_extensions: &'a [&'b str],
    pub enable_independent_blend: bool,
}

pub trait App: Sized {
    fn new(base: &mut BaseApp<Self>) -> Result<Self>;

    fn update(
        &mut self,
        base: &mut BaseApp<Self>,
        image_index: usize,
        delta_time: Duration,
    ) -> Result<()>;

    fn record_raytracing_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize,
    ) -> Result<()>;

    fn record_raster_commands(&self, base: &BaseApp<Self>, image_index: usize) -> Result<()>;

    fn on_recreate_swapchain(&mut self, base: &BaseApp<Self>) -> Result<()>;
}

pub struct BaseApp<A: App> {
    phantom: PhantomData<A>,
    raytracing_enabled: bool,

    pub swapchain: Swapchain,
    pub command_pool: CommandPool,
    pub storage_images: Vec<ImageAndView>,
    pub command_buffers: Vec<CommandBuffer>,
    in_flight_frames: InFlightFrames,

    pub camera: Camera,

    pub context: Context,
    requested_swapchain_format: Option<vk::SurfaceFormatKHR>,
}

impl<A: App> BaseApp<A> {
    fn new(window: &Window, app_name: &str, app_config: AppConfig) -> Result<Self> {
        log::info!("Creating base application");

        let AppConfig {
            enable_raytracing,
            required_instance_extensions,
            enable_independent_blend,
        } = app_config;

        let mut required_extensions = vec!["VK_KHR_swapchain"];
        if enable_raytracing {
            required_extensions.push("VK_KHR_ray_tracing_pipeline");
            required_extensions.push("VK_KHR_acceleration_structure");
            required_extensions.push("VK_KHR_deferred_host_operations");
        }

        let mut context = ContextBuilder::new(window, window)
            .vulkan_version(VERSION_1_3)
            .app_name(app_name)
            .required_instance_extensions(required_instance_extensions)
            .required_device_extensions(&required_extensions)
            .required_device_features(DeviceFeatures {
                ray_tracing_pipeline: enable_raytracing,
                acceleration_structure: enable_raytracing,
                runtime_descriptor_array: enable_raytracing,
                buffer_device_address: enable_raytracing,
                dynamic_rendering: true,
                synchronization2: true,
                independent_blend: enable_independent_blend,
            })
            .with_raytracing_context(enable_raytracing)
            .build()?;

        let command_pool = context.create_command_pool(
            context.graphics_queue_family,
            Some(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
        )?;

        let swapchain = Swapchain::new(
            &context,
            window.inner_size().width,
            window.inner_size().height,
        )?;

        let storage_images = if enable_raytracing {
            create_storage_images(&mut context, swapchain.extent, swapchain.images.len())?
        } else {
            vec![]
        };

        let command_buffers = create_command_buffers(&command_pool, &swapchain)?;
        let in_flight_frames = InFlightFrames::new(&context, IN_FLIGHT_FRAMES)?;

        let camera = Camera::new(
            glam::Vec3::Z,
            glam::Vec3::NEG_Z,
            60.0,
            window.inner_size().width as f32 / window.inner_size().height as f32,
            0.1,
            1000.0,
        );

        Ok(Self {
            phantom: PhantomData,
            raytracing_enabled: enable_raytracing,

            context,
            command_pool,
            swapchain,
            storage_images,
            command_buffers,
            in_flight_frames,

            camera,
            requested_swapchain_format: None,
        })
    }

    pub fn request_swapchain_format_change(&mut self, format: vk::SurfaceFormatKHR) {
        self.requested_swapchain_format = Some(format);
    }

    fn recreate_swapchain(
        &mut self,
        width: u32,
        height: u32,
        format: Option<vk::SurfaceFormatKHR>,
    ) -> Result<()> {
        log::debug!("Recreating the swapchain");

        self.wait_for_gpu()?;

        self.swapchain
            .update(&self.context, width, height, format)?;

        if self.raytracing_enabled {
            let storage_images = create_storage_images(
                &mut self.context,
                self.swapchain.extent,
                self.swapchain.images.len(),
            )?;
            let _ = std::mem::replace(&mut self.storage_images, storage_images);
        }

        self.camera.aspect_ratio = width as f32 / height as f32;

        Ok(())
    }

    pub fn wait_for_gpu(&self) -> Result<()> {
        self.context.device_wait_idle()
    }

    fn draw(
        &mut self,
        window: &Window,
        base_app: &mut A,
        frame_stats: &mut FrameStats,
    ) -> Result<bool> {
        self.in_flight_frames.next();
        self.in_flight_frames.fence().wait(None)?;

        let gpu_time = (frame_stats.total_frame_count >= IN_FLIGHT_FRAMES)
            .then(|| self.in_flight_frames.gpu_frame_time_ms())
            .transpose()?
            .unwrap_or_default();
        frame_stats.set_gpu_time_time(gpu_time);
        frame_stats.tick();

        let next_image_result = self
            .swapchain
            .acquire_next_image(u64::MAX, self.in_flight_frames.image_available_semaphore());
        let image_index = match next_image_result {
            Ok(AcquiredImage { index, .. }) => index as usize,
            Err(err) => match err.downcast_ref::<vk::Result>() {
                Some(&vk::Result::ERROR_OUT_OF_DATE_KHR) => return Ok(true),
                _ => panic!("Error while acquiring next image: {}", err),
            },
        };
        self.in_flight_frames.fence().reset()?;

        base_app.update(self, image_index, frame_stats.frame_time)?;

        self.record_command_buffer(image_index, base_app)?;

        let command_buffer = &self.command_buffers[image_index];
        self.context.graphics_queue.submit(
            command_buffer,
            Some(SemaphoreSubmitInfo {
                semaphore: self.in_flight_frames.image_available_semaphore(),
                stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            }),
            Some(SemaphoreSubmitInfo {
                semaphore: self.in_flight_frames.render_finished_semaphore(),
                stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
            }),
            self.in_flight_frames.fence(),
        )?;

        let signal_semaphores = [self.in_flight_frames.render_finished_semaphore()];
        let present_result = self.swapchain.queue_present(
            image_index as _,
            &signal_semaphores,
            &self.context.present_queue,
        );
        match present_result {
            Ok(true) => return Ok(true),
            Err(err) => match err.downcast_ref::<vk::Result>() {
                Some(&vk::Result::ERROR_OUT_OF_DATE_KHR) => return Ok(true),
                _ => panic!("Failed to present queue: {}", err),
            },
            _ => {}
        }

        Ok(false)
    }

    fn record_command_buffer(&mut self, image_index: usize, base_app: &A) -> Result<()> {
        self.command_buffers[image_index].reset()?;
        self.command_buffers[image_index].begin(None)?;
        self.command_buffers[image_index]
            .reset_all_timestamp_queries_from_pool(self.in_flight_frames.timing_query_pool());
        self.command_buffers[image_index].write_timestamp(
            vk::PipelineStageFlags2::NONE,
            self.in_flight_frames.timing_query_pool(),
            0,
        );

        if self.raytracing_enabled {
            base_app.record_raytracing_commands(
                self,
                &self.command_buffers[image_index],
                image_index,
            )?;
            let storage_image = &self.storage_images[image_index].image;

            self.command_buffers[image_index].pipeline_image_barriers(&[
                ImageBarrier {
                    image: &self.swapchain.images[image_index],
                    old_layout: vk::ImageLayout::UNDEFINED,
                    new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    src_access_mask: vk::AccessFlags2::empty(),
                    dst_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                    src_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                    dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                },
                ImageBarrier {
                    image: storage_image,
                    old_layout: vk::ImageLayout::GENERAL,
                    new_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    src_access_mask: vk::AccessFlags2::SHADER_WRITE,
                    dst_access_mask: vk::AccessFlags2::TRANSFER_READ,
                    src_stage_mask: vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                    dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                },
            ]);

            self.command_buffers[image_index].copy_image(
                storage_image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                &self.swapchain.images[image_index],
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );

            self.command_buffers[image_index].pipeline_image_barriers(&[
                ImageBarrier {
                    image: &self.swapchain.images[image_index],
                    old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                    dst_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                    src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                    dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                },
                ImageBarrier {
                    image: storage_image,
                    old_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    new_layout: vk::ImageLayout::GENERAL,
                    src_access_mask: vk::AccessFlags2::TRANSFER_READ,
                    dst_access_mask: vk::AccessFlags2::SHADER_WRITE,
                    src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                    dst_stage_mask: vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                },
            ]);
        } else {
            self.command_buffers[image_index].pipeline_image_barriers(&[ImageBarrier {
                image: &self.swapchain.images[image_index],
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                src_access_mask: vk::AccessFlags2::empty(),
                dst_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                src_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            }]);
        }

        base_app.record_raster_commands(self, image_index)?;

        self.command_buffers[image_index].pipeline_image_barriers(&[ImageBarrier {
            image: &self.swapchain.images[image_index],
            old_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            src_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            dst_access_mask: vk::AccessFlags2::empty(),
            src_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        }]);

        self.command_buffers[image_index].write_timestamp(
            vk::PipelineStageFlags2::TOP_OF_PIPE,
            self.in_flight_frames.timing_query_pool(),
            1,
        );

        self.command_buffers[image_index].end()?;

        Ok(())
    }
}

pub fn run<A: App + 'static>(
    app_name: &str,
    width: u32,
    height: u32,
    app_config: AppConfig,
) -> Result<()> {
    TermLogger::init(
        simplelog::LevelFilter::Debug,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;

    log::debug!("Creating window and event loop");
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = WindowBuilder::new()
        .with_title(app_name)
        .with_inner_size(PhysicalSize::new(width, height))
        .build(&event_loop)?;

    let mut base_app = BaseApp::new(&window, app_name, app_config)?;
    let mut app = A::new(&mut base_app)?;

    let mut camera_controls = CameraControls::default();
    let mut is_swapchain_dirty = false;
    let mut last_frame = Instant::now();
    let mut frame_stats = FrameStats::default();

    event_loop.run(move |event, ewlt| {
        let app = &mut app;
        camera_controls.handle_event(&event);

        match event {
            Event::NewEvents(_) => {
                let now = Instant::now();
                let frame_time = now - last_frame;
                last_frame = now;

                frame_stats.set_frame_time(frame_time);
                camera_controls = camera_controls.reset();
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(..) => {
                    is_swapchain_dirty = true;
                }
                WindowEvent::CloseRequested => ewlt.exit(),
                _ => {}
            },
            Event::AboutToWait => {
                if is_swapchain_dirty || base_app.requested_swapchain_format.is_some() {
                    let dimensions = window.inner_size();
                    let format = base_app.requested_swapchain_format.take();

                    if dimensions.width > 0 && dimensions.height > 0 {
                        base_app
                            .recreate_swapchain(dimensions.width, dimensions.height, format)
                            .expect("failed to create swapchain")
                    } else {
                        return;
                    }
                }

                base_app.camera = base_app
                    .camera
                    .update(&camera_controls, frame_stats.frame_time);

                is_swapchain_dirty = base_app
                    .draw(&window, app, &mut frame_stats)
                    .expect("failed to draw");
            }
            Event::LoopExiting => base_app
                .wait_for_gpu()
                .expect("failed to wait for gpu to finish work"),
            _ => {}
        }
    })?;

    Ok(())
}

fn create_storage_images(
    context: &mut Context,
    extent: vk::Extent2D,
    count: usize,
) -> Result<Vec<ImageAndView>> {
    let mut images = Vec::with_capacity(count);

    for _ in 0..count {
        let image = context.create_image(
            vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::STORAGE,
            MemoryLocation::GpuOnly,
            vk::Format::R8G8B8A8_UNORM,
            extent.width,
            extent.height,
        )?;

        let view = image.create_image_view(vk::ImageAspectFlags::COLOR)?;

        context.execute_one_time_commands(|cmd_buffer| {
            cmd_buffer.pipeline_image_barriers(&[ImageBarrier {
                image: &image,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::GENERAL,
                src_access_mask: vk::AccessFlags2::NONE,
                dst_access_mask: vk::AccessFlags2::SHADER_WRITE,
                src_stage_mask: vk::PipelineStageFlags2::NONE,
                dst_stage_mask: vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
            }]);
        })?;

        images.push(ImageAndView { image, view })
    }

    Ok(images)
}

fn create_command_buffers(pool: &CommandPool, swapchain: &Swapchain) -> Result<Vec<CommandBuffer>> {
    pool.allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, swapchain.images.len() as _)
}

pub struct ImageAndView {
    pub image: Image,
    pub view: ImageView,
}

struct InFlightFrames {
    per_frames: Vec<PerFrame>,
    current_frame: usize,
}

struct PerFrame {
    image_available_semaphore: Semaphore,
    render_finished_semaphore: Semaphore,
    fence: Fence,
    timing_query_pool: TimestampQueryPool<2>,
}

impl InFlightFrames {
    fn new(context: &Context, frame_count: u32) -> Result<Self> {
        let sync_objects = (0..frame_count)
            .map(|_i| {
                let image_available_semaphore = context.create_semaphore()?;
                let render_finished_semaphore = context.create_semaphore()?;
                let fence = context.create_fence(Some(vk::FenceCreateFlags::SIGNALED))?;

                let timing_query_pool = context.create_timestamp_query_pool()?;

                Ok(PerFrame {
                    image_available_semaphore,
                    render_finished_semaphore,
                    fence,
                    timing_query_pool,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            per_frames: sync_objects,
            current_frame: 0,
        })
    }

    fn next(&mut self) {
        self.current_frame = (self.current_frame + 1) % self.per_frames.len();
    }

    fn image_available_semaphore(&self) -> &Semaphore {
        &self.per_frames[self.current_frame].image_available_semaphore
    }

    fn render_finished_semaphore(&self) -> &Semaphore {
        &self.per_frames[self.current_frame].render_finished_semaphore
    }

    fn fence(&self) -> &Fence {
        &self.per_frames[self.current_frame].fence
    }

    fn timing_query_pool(&self) -> &TimestampQueryPool<2> {
        &self.per_frames[self.current_frame].timing_query_pool
    }

    fn gpu_frame_time_ms(&self) -> Result<Duration> {
        let result = self.timing_query_pool().wait_for_all_results()?;
        let time = Duration::from_nanos(result[1].saturating_sub(result[0]));

        Ok(time)
    }
}

#[derive(Debug)]
struct FrameStats {
    previous_frame_time: Duration,
    frame_time: Duration,
    cpu_time: Duration,
    gpu_time: Duration,

    frame_time_ms_log: Queue<f32>,
    cpu_time_ms_log: Queue<f32>,
    gpu_time_ms_log: Queue<f32>,

    total_frame_count: u32,
    frame_count: u32,

    fps_counter: u32,
    timer: Duration,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self {
            previous_frame_time: Default::default(),
            frame_time: Default::default(),
            cpu_time: Default::default(),
            gpu_time: Default::default(),
            frame_time_ms_log: Queue::new(FrameStats::MAX_LOG_SIZE),
            cpu_time_ms_log: Queue::new(FrameStats::MAX_LOG_SIZE),
            gpu_time_ms_log: Queue::new(FrameStats::MAX_LOG_SIZE),
            total_frame_count: Default::default(),
            frame_count: Default::default(),
            fps_counter: Default::default(),
            timer: Default::default(),
        }
    }
}

impl FrameStats {
    const ONE_SEC: Duration = Duration::from_secs(1);
    const MAX_LOG_SIZE: usize = 1000;

    fn tick(&mut self) {
        self.cpu_time = self.previous_frame_time.saturating_sub(self.gpu_time);

        self.frame_time_ms_log
            .push(self.previous_frame_time.as_millis() as _);
        self.cpu_time_ms_log.push(self.cpu_time.as_millis() as _);
        self.gpu_time_ms_log.push(self.gpu_time.as_millis() as _);

        self.total_frame_count += 1;
        self.frame_count += 1;
        self.timer += self.frame_time;

        if self.timer > FrameStats::ONE_SEC {
            self.fps_counter = self.frame_count;
            self.frame_count = 0;
            self.timer -= FrameStats::ONE_SEC;
        }
    }

    fn set_frame_time(&mut self, frame_time: Duration) {
        self.previous_frame_time = self.frame_time;
        self.frame_time = frame_time;
    }

    fn set_gpu_time_time(&mut self, gpu_time: Duration) {
        self.gpu_time = gpu_time;
    }
}

#[derive(Debug)]
struct Queue<T>(Vec<T>, usize);

impl<T> Queue<T> {
    fn new(max_size: usize) -> Self {
        Self(Vec::with_capacity(max_size), max_size)
    }

    fn push(&mut self, value: T) {
        if self.0.len() == self.1 {
            self.0.remove(0);
        }
        self.0.push(value);
    }
}
