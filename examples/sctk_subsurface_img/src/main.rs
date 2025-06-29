use cctk::sctk::reexports::client::protocol::wl_shm;
use iced::{
    platform_specific::shell::subsurface_widget::{
        self, Shmbuf, SubsurfaceBuffer,
    },
    window::{self, Id, Settings},
    Element, Subscription, Task,
};
use image::{ImageReader, Pixel};
use rustix::{io::Errno, shm::ShmOFlags};
use std::{
    env,
    os::fd::OwnedFd,
    path::Path,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

fn main() -> iced::Result {
    let args = env::args();
    if args.len() != 2 {
        eprintln!("usage: sctk_subsurface_img [image path]");
        return Ok(());
    }
    let path = args.skip(1).next().unwrap();
    if !Path::new(&path).exists() {
        eprintln!("File `{path}` not found.");
        return Ok(());
    }

    iced::daemon(
        SubsurfaceApp::title,
        SubsurfaceApp::update,
        SubsurfaceApp::view,
    )
    .subscription(SubsurfaceApp::subscription)
    .run_with(|| SubsurfaceApp::new(path))
}

#[derive(Debug, Clone)]
struct SubsurfaceApp {
    path: String,
    buffer: SubsurfaceBuffer,
    use_subsurface: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Id(Id),
    Toggle,
}

impl SubsurfaceApp {
    fn new(path: String) -> (SubsurfaceApp, Task<Message>) {
        let img = ImageReader::open(&path)
            .unwrap()
            .decode()
            .unwrap()
            .to_rgba8();
        let fd = create_memfile().unwrap();
        for pixel in img.pixels() {
            let [r, g, b, a] = <[u8; 4]>::try_from(pixel.channels()).unwrap();
            rustix::io::write(&fd, &[b, g, r, a]).unwrap();
        }
        let shmbuf = Shmbuf {
            fd,
            offset: 0,
            width: img.width() as i32,
            height: img.height() as i32,
            stride: img.width() as i32 * 4,
            format: wl_shm::Format::Xrgb8888,
        };
        let buffer = SubsurfaceBuffer::new(Arc::new(shmbuf.into())).0;

        (
            SubsurfaceApp {
                path,
                buffer,
                use_subsurface: true,
            },
            iced::window::open(Settings {
                ..Default::default()
            })
            .1
            .map(Message::Id),
        )
    }

    fn title(&self, _id: window::Id) -> String {
        String::from("SubsurfaceApp")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Id(_) => {}
            Message::Toggle => {
                self.use_subsurface = !self.use_subsurface;
                if self.use_subsurface {
                    println!("Using subsurface");
                } else {
                    println!("Using image widget");
                }
            }
        }
        Task::none()
    }

    fn view(&self, _id: window::Id) -> Element<Message> {
        let image: Element<_> = if self.use_subsurface {
            subsurface_widget::Subsurface::new(self.buffer.clone())
                .content_fit(iced::ContentFit::None)
                .into()
        } else {
            iced::widget::image::Image::new(&self.path)
                .content_fit(iced::ContentFit::None)
                .into()
        };
        iced::widget::scrollable(image).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|evt, _status, _id| match evt {
            iced::Event::Keyboard(iced::keyboard::Event::KeyReleased {
                key,
                ..
            }) => match key {
                iced::keyboard::Key::Named(
                    iced::keyboard::key::Named::Space,
                ) => Some(Message::Toggle),
                _ => None,
            },
            _ => None,
        })
    }
}

fn create_memfile() -> rustix::io::Result<OwnedFd> {
    loop {
        let flags = ShmOFlags::CREATE | ShmOFlags::EXCL | ShmOFlags::RDWR;

        let time = SystemTime::now();
        let name = format!(
            "/iced-sctk-{}",
            time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
        );

        match rustix::io::retry_on_intr(|| {
            rustix::shm::shm_open(&name, flags, 0600.into())
        }) {
            Ok(fd) => match rustix::shm::shm_unlink(&name) {
                Ok(_) => return Ok(fd),
                Err(errno) => {
                    return Err(errno.into());
                }
            },
            Err(Errno::EXIST) => {
                continue;
            }
            Err(err) => return Err(err.into()),
        }
    }
}
