use nfc1::target_info::TargetInfo;
use nfc1::{BaudRate, Context, Device, Modulation, ModulationType, Target};
use std::error::Error;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use watch::WatchReceiver;
use wgpu::Color;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Fullscreen, Window, WindowId};

const CORRECT_UID: [u8; 4] = [0x6A, 0x69, 0x33, 0x69];
const KEY_A: [u8; 6] = [0x43, 0x75, 0x5A, 0x74, 0x30, 0x4D];
const CORRECT_AUTH_DATA: [u8; 16] = [
    0x44, 0x61, 0x74, 0x61, 0x55, 0x73, 0x65, 0x64, 0x46, 0x6F, 0x72, 0x41, 0x75, 0x74, 0x68, 0x00,
];
const BLOCK_TO_READ: u8 = 4;
const POLLING_RETRY_TIME_MS: u64 = 1000;

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
                    Window::default_attributes()
                        .with_title("Card Status")
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

fn read_block(device: &mut Device, block: u8) -> Result<Vec<u8>, CardError> {
    authenticate(device, block)?;
    mifare_read(device, block)
}

fn authenticate(device: &mut Device, block: u8) -> Result<(), CardError> {
    println!("Attempt authentication with key {:02X?}...", KEY_A);

    // Build raw authentication frame
    let mut frame = Vec::with_capacity(12);
    frame.push(0x60);
    frame.push(block);
    frame.extend_from_slice(&KEY_A);
    frame.extend_from_slice(&CORRECT_UID);

    let bytes = device.initiator_transceive_bytes(&frame, 254, nfc1::Timeout::Default)?;

    // Fingerprinting Mifare Classic cards (empty response = MFC)
    match bytes.len() {
        0 => Ok(()),
        _ => Err(CardError::UnsupportedCard),
    }
}

fn mifare_read(device: &mut Device, block: u8) -> Result<Vec<u8>, CardError> {
    let cmd = [0x30, block];
    let bytes = device.initiator_transceive_bytes(&cmd, 254, nfc1::Timeout::Default)?;

    Ok(bytes)
}

#[derive(Debug)]
enum CardError {
    NotFound,
    UnsupportedCard,
    UnknownError,
    NfcError(nfc1::Error),
}

impl From<nfc1::Error> for CardError {
    fn from(value: nfc1::Error) -> Self {
        CardError::NfcError(value)
    }
}

fn poll_for_card(device: &mut Device, modulation: &Modulation) -> Result<(), CardError> {
    // println!("Polling for target...");
    match device.initiator_select_passive_target(&modulation) {
        Ok(Target {
            target_info: TargetInfo::Iso14443a(info),
            modulation: _,
        }) => {
            if info.uid_len == 0 {
                // eprintln!("No card found.");
                Err(CardError::NotFound)
            } else {
                let uid_slice = &info.uid[..info.uid_len];
                println!("UID (Hex): {:02X?}", uid_slice);
                Ok(())
            }
        }
        Ok(_target) => Err(CardError::UnsupportedCard),
        Err(error) => {
            eprintln!("Unknown error occurred: {}", error);
            Err(CardError::UnknownError)
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut ctx = Context::new()?;

    let mut device = ctx.open()?;
    println!("NFC Device opened: {}", device.name());

    let modulation = Modulation {
        modulation_type: ModulationType::Iso14443a,
        baud_rate: BaudRate::Baud106,
    };

    const YELLOW: Color = Color {
        r: (255.0),
        g: (255.0),
        b: (0.0),
        a: (0.0),
    };

    // Create channel between 2 threads to share information.
    let (sender, receiver) = watch::channel(YELLOW);

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        state: None,
        receiver,
    };

    std::thread::spawn(move || {
        loop {
            sender.send(YELLOW);

            while let Err(_) = poll_for_card(&mut device, &modulation) {
                thread::sleep(Duration::from_millis(POLLING_RETRY_TIME_MS));
            }

            match read_block(&mut device, BLOCK_TO_READ) {
                Ok(bytes) => {
                    println!("Received bytes: {:02X?}", bytes);
                    if bytes == CORRECT_AUTH_DATA {
                        sender.send(Color::GREEN);
                    } else {
                        sender.send(Color::RED);
                    }
                }
                Err(CardError::NfcError(nfc1::Error::MifareAuthFailed)) => {
                    sender.send(Color::RED);
                }
                Err(error) => {
                    eprintln!("Error: {:?}", error);
                }
            };

            thread::sleep(Duration::from_millis(POLLING_RETRY_TIME_MS * 3));
        }
    });

    event_loop.run_app(&mut app).unwrap();
    Ok(())
}
