use crate::gui::Gui;
use std::time::{Duration, Instant};

use anyhow::Result;
use app::{App, BaseApp};
use camera::CameraControls;
use egui::TextureId;
use simplelog::TermLogger;
use utils::Queue;
use vulkan::{ash::vk, Context, Fence, Image, ImageView, Semaphore, TimestampQueryPool};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

pub mod app;
pub mod gui;

mod camera;
mod utils;

const IN_FLIGHT_FRAMES: u32 = 2;

#[derive(Debug, Default)]
pub struct AppConfig<'a, 'b> {
    pub enable_raytracing: bool,
    pub required_instance_extensions: &'a [&'b str],
    pub enable_independent_blend: bool,
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
    let mut ui = A::Gui::new(&base_app)?;

    let mut camera_controls = CameraControls::default();
    let mut is_swapchain_dirty = false;
    let mut last_frame = Instant::now();
    let mut frame_stats = FrameStats::default();

    event_loop.run(move |event, ewlt| {
        let app = &mut app;
        camera_controls = camera_controls.handle_event(&event);

        match event {
            Event::NewEvents(_) => {
                let now = Instant::now();
                let frame_time = now - last_frame;
                last_frame = now;

                frame_stats.set_frame_time(frame_time);
                camera_controls = camera_controls.reset();
            }
            Event::WindowEvent { event, .. } => {
                base_app.gui_context.handle_event(&window, &event);
                match event {
                    WindowEvent::Resized(..) => {
                        is_swapchain_dirty = true;
                    }
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(KeyCode::F3),
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    } => base_app.stats_display_mode = base_app.stats_display_mode.next(),
                    WindowEvent::CloseRequested => ewlt.exit(),
                    _ => {}
                }
            }
            Event::AboutToWait => {
                if is_swapchain_dirty || base_app.requested_swapchain_format.is_some() {
                    let dimensions = window.inner_size();
                    let format = base_app.requested_swapchain_format.take();

                    if dimensions.width > 0 && dimensions.height > 0 {
                        base_app
                            .recreate_swapchain(dimensions.width, dimensions.height, format)
                            .expect("failed to recreate swapchain on the base app");
                        app.on_recreate_swapchain(&base_app)
                            .expect("failed to recreate swapchain in the user app");
                    } else {
                        return;
                    }
                }

                base_app.camera = base_app
                    .camera
                    .update(&camera_controls, frame_stats.frame_time);

                is_swapchain_dirty = base_app
                    .draw(&window, app, &mut ui, &mut frame_stats)
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
    gui_textures_to_free: Vec<TextureId>,
}

impl InFlightFrames {
    fn new(context: &Context, frame_count: u32) -> Result<Self> {
        let sync_objects = (0..frame_count)
            .map(|_i| {
                let image_available_semaphore = context.create_semaphore()?;
                let render_finished_semaphore = context.create_semaphore()?;
                let fence = context.create_fence(Some(vk::FenceCreateFlags::SIGNALED))?;

                let timing_query_pool = context.create_timestamp_query_pool()?;
                let gui_textures_to_free = Vec::new();

                Ok(PerFrame {
                    image_available_semaphore,
                    render_finished_semaphore,
                    fence,
                    timing_query_pool,
                    gui_textures_to_free,
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

    fn gui_textures_to_free(&self) -> &[TextureId] {
        &self.per_frames[self.current_frame].gui_textures_to_free
    }

    fn set_gui_textures_to_free(&mut self, ids: Vec<TextureId>) {
        self.per_frames[self.current_frame].gui_textures_to_free = ids;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatsDisplayMode {
    None,
    Basic,
    Full,
}

impl StatsDisplayMode {
    fn next(self) -> Self {
        match self {
            Self::None => Self::Basic,
            Self::Basic => Self::Full,
            Self::Full => Self::None,
        }
    }
}
