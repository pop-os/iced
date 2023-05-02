use futures_channel::mpsc;
use iced::{
    futures::{FutureExt, SinkExt},
    platform_specific::shell::subsurface_widget::{Shmbuf, SubsurfaceBuffer},
};
use iced_runtime::futures::subscription;
use rustix::{io::Errno, shm::ShmOFlags};
use sctk::{
    reexports::{
        calloop_wayland_source::WaylandSource,
        client::{
            delegate_noop,
            globals::registry_queue_init,
            protocol::{wl_buffer::WlBuffer, wl_shm},
            Connection,
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    shm::{Shm, ShmHandler},
};
use std::{
    os::fd::OwnedFd,
    sync::Arc,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone)]
pub enum Event {
    RedBuffer(SubsurfaceBuffer),
}

struct AppData {
    registry_state: RegistryState,
    shm_state: Shm,
}

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    sctk::registry_handlers!();
}

impl ShmHandler for AppData {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

pub fn subscription(connection: &Connection) -> iced::Subscription<Event> {
    let connection = connection.clone();
    subscription::Subscription::run_with_id(
        "wayland-sub",
        async { start(connection).await }.flatten_stream(),
    )
}

async fn start(conn: Connection) -> mpsc::Receiver<Event> {
    let (mut sender, receiver) = mpsc::channel(20);

    let (globals, event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let mut app_data = AppData {
        registry_state: RegistryState::new(&globals),
        shm_state: Shm::bind(&globals, &qh).unwrap(),
    };

    let fd = create_memfile().unwrap();
    rustix::io::write(&fd, &[0, 255, 0, 255]).unwrap();

    let shmbuf = Shmbuf {
        fd,
        offset: 0,
        width: 1,
        height: 1,
        stride: 4,
        format: wl_shm::Format::Xrgb8888,
    };

    let buffer = SubsurfaceBuffer::new(Arc::new(shmbuf.into())).0;
    let _ = sender.send(Event::RedBuffer(buffer)).await;

    thread::spawn(move || {
        let mut event_loop = calloop::EventLoop::try_new().unwrap();
        WaylandSource::new(conn, event_queue)
            .insert(event_loop.handle())
            .unwrap();
        loop {
            event_loop.dispatch(None, &mut app_data).unwrap();
        }
    });

    receiver
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

delegate_noop!(AppData: ignore WlBuffer);
sctk::delegate_registry!(AppData);
sctk::delegate_shm!(AppData);
