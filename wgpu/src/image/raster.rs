use crate::core::Size;
use crate::core::image;
use crate::graphics;
use crate::image::atlas::{self, Atlas};

use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::{Arc, Weak};

pub type Image = graphics::image::Buffer;

/// Entry in cache corresponding to an image handle
#[derive(Debug)]
pub enum Memory {
    /// Image data on host
    Host(Image),
    Error(image::Error),
}

impl Memory {
    pub fn load(handle: &image::Handle) -> Self {
        match graphics::image::load(handle) {
            Ok(image) => Self::Host(image),
            Err(error) => Self::Error(error),
        }
    }

    pub fn dimensions(&self) -> Size<u32> {
        match self {
            Memory::Host(image) => {
                let (width, height) = image.dimensions();

                Size::new(width, height)
            }
            Memory::Error(_) => Size::new(1, 1),
        }
    }
}

#[derive(Debug, Default)]
pub struct Cache {
    images: FxHashMap<image::Id, Memory>,
    entries: FxHashMap<UploadKey, Device>,
    image_hits: FxHashSet<image::Id>,
    entry_hits: FxHashSet<UploadKey>,
    should_trim: bool,
}

impl Cache {
    pub fn get_mut(&mut self, handle: &image::Handle) -> Option<&mut Memory> {
        let _ = self.image_hits.insert(handle.id());

        self.images.get_mut(&handle.id())
    }

    pub fn insert(&mut self, handle: &image::Handle, memory: Memory) {
        let _ = self.images.insert(handle.id(), memory);
        let _ = self.image_hits.insert(handle.id());

        self.should_trim = true;
    }

    pub fn contains(&self, handle: &image::Handle) -> bool {
        self.images.contains_key(&handle.id())
    }

    pub fn get_entry(
        &mut self,
        handle: &image::Handle,
        target_size: Size<u32>,
    ) -> Option<&mut Device> {
        let key = UploadKey::new(handle.id(), target_size);
        let _ = self.image_hits.insert(handle.id());
        let _ = self.entry_hits.insert(key);

        self.entries.get_mut(&key)
    }

    pub fn insert_entry(
        &mut self,
        handle: &image::Handle,
        target_size: Size<u32>,
        device: Device,
    ) {
        let key = UploadKey::new(handle.id(), target_size);
        let _ = self.image_hits.insert(handle.id());
        let _ = self.entry_hits.insert(key);
        let _ = self.entries.insert(key, device);

        self.should_trim = true;
    }

    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut wgpu::util::StagingBelt,
        handle: &image::Handle,
        target_size: Size<u32>,
        atlas: &mut Atlas,
    ) -> Option<&Device> {
        let key = UploadKey::new(handle.id(), target_size);
        let _ = self.image_hits.insert(handle.id());
        let _ = self.entry_hits.insert(key);

        if self.entries.contains_key(&key) {
            return self.entries.get(&key);
        }

        let memory = self.get_mut(handle)?;
        let Memory::Host(image) = memory else {
            return None;
        };

        let upload_size = upload_size(image, target_size);
        let entry = atlas.upload(
            device,
            encoder,
            belt,
            upload_size.width,
            upload_size.height,
            resized_bytes(image, upload_size)
                .as_deref()
                .unwrap_or(image),
        )?;

        self.insert_entry(
            handle,
            upload_size,
            Device {
                entry,
                bind_group: None,
                allocation: None,
            },
        );

        self.entries.get(&key)
    }

    pub fn trim(
        &mut self,
        atlas: &mut Atlas,
        on_drop: impl Fn(Arc<wgpu::BindGroup>),
    ) {
        // Only trim if new entries have landed in the `Cache`
        if !self.should_trim {
            return;
        }

        let image_hits = &self.image_hits;
        let entry_hits = &self.entry_hits;

        self.images.retain(|id, _| image_hits.contains(id));

        self.entries.retain(|key, device| {
            if device
                .allocation
                .as_ref()
                .is_some_and(|allocation| allocation.strong_count() > 0)
            {
                return true;
            }

            let retain = entry_hits.contains(key);

            if !retain {
                log::debug!("Dropping image allocation: {key:?}");

                if let Some(bind_group) = device.bind_group.take() {
                    on_drop(bind_group);
                } else {
                    atlas.remove(&device.entry);
                }
            }

            retain
        });

        self.image_hits.clear();
        self.entry_hits.clear();
        self.should_trim = false;
    }
}

#[derive(Debug)]
pub struct Device {
    pub entry: atlas::Entry,
    pub bind_group: Option<Arc<wgpu::BindGroup>>,
    pub allocation: Option<Weak<image::Memory>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct UploadKey {
    image_id: image::Id,
    width: u32,
    height: u32,
}

impl UploadKey {
    fn new(image_id: image::Id, target_size: Size<u32>) -> Self {
        Self {
            image_id,
            width: target_size.width.max(1),
            height: target_size.height.max(1),
        }
    }
}

fn upload_size(image: &Image, target_size: Size<u32>) -> Size<u32> {
    Size::new(
        target_size.width.max(1).min(image.width()),
        target_size.height.max(1).min(image.height()),
    )
}

fn resized_bytes(image: &Image, target_size: Size<u32>) -> Option<Vec<u8>> {
    if target_size.width == image.width()
        && target_size.height == image.height()
    {
        return None;
    }

    Some(
        graphics::image::imageops::resize(
            image,
            target_size.width,
            target_size.height,
            graphics::image::imageops::FilterType::Lanczos3,
        )
        .into_raw(),
    )
}
