use pyo3::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use watch::WatchReceiver;
use wgpu::Color;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::platform::wayland::EventLoopBuilderExtWayland;
use winit::window::{Fullscreen, Window, WindowId};

struct State {
    device: wgpu::Device,
    size: winit::dpi::PhysicalSize<u32>,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    window: Arc<Window>,
}

impl State {
    async fn new(window: Arc<Window>) -> State {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let state = State {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
        };

        // Configure surface for the first time
        state.configure_surface();

        state
    }

    fn get_window(&self) -> &Window {
        &self.window
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            // Request compatibility with the sRGB-format texture view we‘re going to create later.
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();
    }

    fn render(&mut self, color: Color) {
        // Create texture view
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        // Create the renderpass which will clear the screen.
        let renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // If you wanted to call any drawing commands, they would go here.

        // End the renderpass.
        drop(renderpass);

        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }
}

struct App {
    state: Option<State>,
    receiver: WatchReceiver<Color>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window object
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes().with_title("Reader Status Indicator")
                    .with_fullscreen(Some(Fullscreen::Borderless(None))),
                )
                .unwrap(),
        );

        let state = pollster::block_on(State::new(window.clone()));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("Closing Window...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render(self.receiver.get());
                // Emits a new redraw requested event.
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                // Reconfigures the size of the surface. We do iot re-render
                // here as this event is always followed up by redraw request.
                state.resize(size);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => {
                println!("Closing Window...");
                event_loop.exit();
            }
            _ => (),
        }
    }
}

fn run_event_loop(receiver: WatchReceiver<Color>, closed: Arc<AtomicBool>) {
    let mut builder = EventLoop::builder();
    builder.with_any_thread(true);

    let event_loop = builder.build().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        state: None,
        receiver,
    };

    event_loop.run_app(&mut app).unwrap();

    closed.store(true, Ordering::SeqCst);
}

#[pyclass]
pub struct WindowHandle {
    sender: watch::WatchSender<Color>,
    closed: Arc<AtomicBool>,
}

#[pymethods]
impl WindowHandle {
    fn set_color(&self, r: f64, g: f64, b: f64, a: f64) -> PyResult<()> {
        let color = Color { r, g, b, a };
        self.sender.send(color);
        Ok(())
    }

    fn closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

#[pyfunction]
fn init_window() -> PyResult<WindowHandle> {
    const YELLOW: Color = Color {
        r: (255.0),
        g: (255.0),
        b: (0.0),
        a: (0.0),
    };
    let (sender, receiver) = watch::channel(YELLOW);
    let closed = Arc::new(AtomicBool::new(false));
    let thread_closed = closed.clone();

    std::thread::spawn(move || {
        run_event_loop(receiver, thread_closed);
    });

    Ok(WindowHandle { sender, closed })
}

#[pymodule]
fn reader_status_indicator(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<WindowHandle>()?;
    m.add_function(wrap_pyfunction!(init_window, m)?)?;

    Ok(())
}
