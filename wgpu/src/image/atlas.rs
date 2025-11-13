pub mod entry;

mod allocation;
mod allocator;
mod layer;

pub use allocation::Allocation;
pub use entry::Entry;
pub use layer::Layer;

use allocator::Allocator;

// Atlas tile size: 4096x4096 provides good balance between:
// - Fewer fragments for large images (better performance)
// - GPU buffer size limits (~67MB per tile, well under 256MB limit)
// https://docs.rs/wgpu/latest/wgpu/struct.Limits.html#structfield.max_buffer_size
pub const SIZE: u32 = 4096;

use crate::core::Size;
use crate::graphics::color;

use std::sync::Arc;

#[derive(Debug)]
pub struct Atlas {
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    texture_bind_group: wgpu::BindGroup,
    texture_layout: Arc<wgpu::BindGroupLayout>,
    layers: Vec<Layer>,
}

impl Atlas {
    pub fn new(
        device: &wgpu::Device,
        backend: wgpu::Backend,
        texture_layout: Arc<wgpu::BindGroupLayout>,
    ) -> Self {
        let layers = match backend {
            // On the GL backend we start with 2 layers, to help wgpu figure
            // out that this texture is `GL_TEXTURE_2D_ARRAY` rather than `GL_TEXTURE_2D`
            // https://github.com/gfx-rs/wgpu/blob/004e3efe84a320d9331371ed31fa50baa2414911/wgpu-hal/src/gles/mod.rs#L371
            wgpu::Backend::Gl => vec![Layer::Empty, Layer::Empty],
            _ => vec![Layer::Empty],
        };

        let extent = wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: layers.len() as u32,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("iced_wgpu::image texture atlas"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if color::GAMMA_CORRECTION {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            },
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        let texture_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("iced_wgpu::image texture atlas bind group"),
                layout: &texture_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                }],
            });

        Atlas {
            texture,
            texture_view,
            texture_bind_group,
            texture_layout,
            layers,
        }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.texture_bind_group
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> Option<Entry> {
        let entry = {
            let current_size = self.layers.len();
            let entry = self.allocate(width, height)?;

            // We grow the internal texture after allocating if necessary
            let new_layers = self.layers.len() - current_size;
            self.grow(new_layers, device, encoder);

            entry
        };

        log::debug!("Allocated atlas entry: {entry:?}");

        match &entry {
            Entry::Contiguous(allocation) => {
                // For contiguous allocations, we can create a single padded buffer
                let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
                let padding = (align - (4 * width) % align) % align;
                let padded_width = (4 * width + padding) as usize;
                let padded_data_size = padded_width * height as usize;

                // Check buffer size BEFORE allocating memory
                let max_buffer_size = device.limits().max_buffer_size as usize;
                if padded_data_size > max_buffer_size {
                    log::error!(
                        "Image {}x{} requires {} bytes buffer, exceeding device limit of {} bytes. Cannot upload.",
                        width,
                        height,
                        padded_data_size,
                        max_buffer_size
                    );
                    // Deallocate the entry since we can't upload
                    self.remove(&entry);
                    return None;
                }

                let mut padded_data = vec![0; padded_data_size];

                for row in 0..height as usize {
                    let offset = row * padded_width;
                    padded_data[offset..offset + 4 * width as usize].copy_from_slice(
                        &data[row * 4 * width as usize..(row + 1) * 4 * width as usize],
                    );
                }

                self.upload_allocation(
                    &padded_data,
                    width,
                    height,
                    padding,
                    0,
                    allocation,
                    device,
                    encoder,
                );
            }
            Entry::Fragmented { fragments, .. } => {
                log::debug!(
                    "Uploading large image {}x{} as {} fragments",
                    width,
                    height,
                    fragments.len()
                );

                let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
                let max_frag_width = fragments.iter()
                    .map(|f| f.allocation.size().width)
                    .max()
                    .unwrap_or(4096);
                let max_frag_height = fragments.iter()
                    .map(|f| f.allocation.size().height)
                    .max()
                    .unwrap_or(4096);

                let max_padding = (align - (4 * max_frag_width) % align) % align;
                let max_padded_width = (4 * max_frag_width + max_padding) as usize;
                let max_buffer_size = max_padded_width * max_frag_height as usize;

                let mut reusable_buffer = vec![0u8; max_buffer_size];

                for fragment in fragments {
                    let (frag_x, frag_y) = fragment.position;
                    let frag_size = fragment.allocation.size();
                    let frag_width = frag_size.width;
                    let frag_height = frag_size.height;

                    if frag_x + frag_width > width || frag_y + frag_height > height {
                        log::error!(
                            "Fragment out of bounds: {}x{} at ({}, {}) exceeds image {}x{}. Skipping fragment.",
                            frag_width, frag_height, frag_x, frag_y, width, height
                        );
                        continue;
                    }

                    let padding = (align - (4 * frag_width) % align) % align;
                    let padded_width = (4 * frag_width + padding) as usize;
                    let padded_data_size = padded_width * frag_height as usize;

                    let fragment_data = &mut reusable_buffer[..padded_data_size];

                    let src_row_size = 4 * frag_width as usize;

                    for row in 0..frag_height as usize {
                        let src_row = frag_y as usize + row;
                        let src_offset = src_row * 4 * width as usize + frag_x as usize * 4;
                        let src_end = src_offset + src_row_size;
                        let dst_offset = row * padded_width;
                        let dst_end = dst_offset + src_row_size;

                        if src_end > data.len() {
                            log::error!(
                                "Fragment upload bounds error: trying to read data[{}..{}] but data length is {}. Skipping fragment.",
                                src_offset, src_end, data.len()
                            );
                            break;
                        }

                        if dst_end > fragment_data.len() {
                            log::error!(
                                "Fragment buffer bounds error: trying to write to [{}..{}] but buffer length is {}. Skipping fragment.",
                                dst_offset, dst_end, fragment_data.len()
                            );
                            break;
                        }

                        fragment_data[dst_offset..dst_end].copy_from_slice(&data[src_offset..src_end]);

                        if padding > 0 {
                            let padding_start = dst_end;
                            let padding_end = dst_offset + padded_width;
                            fragment_data[padding_start..padding_end].fill(0);
                        }
                    }

                    self.upload_allocation(
                        fragment_data,
                        frag_width,
                        frag_height,
                        padding,
                        0,
                        &fragment.allocation,
                        device,
                        encoder,
                    );
                }
            }
        }

        if log::log_enabled!(log::Level::Debug) {
            log::debug!(
                "Atlas layers: {} (busy: {}, allocations: {})",
                self.layer_count(),
                self.layers.iter().filter(|layer| !layer.is_empty()).count(),
                self.layers.iter().map(Layer::allocations).sum::<usize>(),
            );
        }

        Some(entry)
    }

    pub fn remove(&mut self, entry: &Entry) {
        log::debug!("Removing atlas entry: {entry:?}");

        match entry {
            Entry::Contiguous(allocation) => {
                self.deallocate(allocation);
            }
            Entry::Fragmented { fragments, .. } => {
                for fragment in fragments {
                    self.deallocate(&fragment.allocation);
                }
            }
        }
    }

    fn allocate(&mut self, width: u32, height: u32) -> Option<Entry> {
        // Allocate one layer if texture fits perfectly
        if width == SIZE && height == SIZE {
            let mut empty_layers = self
                .layers
                .iter_mut()
                .enumerate()
                .filter(|(_, layer)| layer.is_empty());

            if let Some((i, layer)) = empty_layers.next() {
                *layer = Layer::Full;

                return Some(Entry::Contiguous(Allocation::Full { layer: i }));
            }

            self.layers.push(Layer::Full);

            return Some(Entry::Contiguous(Allocation::Full {
                layer: self.layers.len() - 1,
            }));
        }

        // Split big textures across multiple layers
        if width > SIZE || height > SIZE {
            let mut fragments = Vec::new();
            let mut y = 0;

            while y < height {
                let height = std::cmp::min(height - y, SIZE);
                let mut x = 0;

                while x < width {
                    let width = std::cmp::min(width - x, SIZE);

                    let allocation = self.allocate(width, height)?;

                    if let Entry::Contiguous(allocation) = allocation {
                        fragments.push(entry::Fragment {
                            position: (x, y),
                            allocation,
                        });
                    }

                    x += width;
                }

                y += height;
            }

            return Some(Entry::Fragmented {
                size: Size::new(width, height),
                fragments,
            });
        }

        // Try allocating on an existing layer
        for (i, layer) in self.layers.iter_mut().enumerate() {
            match layer {
                Layer::Empty => {
                    let mut allocator = Allocator::new(SIZE);

                    if let Some(region) = allocator.allocate(width, height) {
                        *layer = Layer::Busy(allocator);

                        return Some(Entry::Contiguous(Allocation::Partial {
                            region,
                            layer: i,
                        }));
                    }
                }
                Layer::Busy(allocator) => {
                    if let Some(region) = allocator.allocate(width, height) {
                        return Some(Entry::Contiguous(Allocation::Partial {
                            region,
                            layer: i,
                        }));
                    }
                }
                Layer::Full => {}
            }
        }

        // Create new layer with atlas allocator
        let mut allocator = Allocator::new(SIZE);

        if let Some(region) = allocator.allocate(width, height) {
            self.layers.push(Layer::Busy(allocator));

            return Some(Entry::Contiguous(Allocation::Partial {
                region,
                layer: self.layers.len() - 1,
            }));
        }

        // We ran out of memory (?)
        None
    }

    fn deallocate(&mut self, allocation: &Allocation) {
        log::debug!("Deallocating atlas: {allocation:?}");

        match allocation {
            Allocation::Full { layer } => {
                self.layers[*layer] = Layer::Empty;
            }
            Allocation::Partial { layer, region } => {
                let layer = &mut self.layers[*layer];

                if let Layer::Busy(allocator) = layer {
                    allocator.deallocate(region);

                    if allocator.is_empty() {
                        *layer = Layer::Empty;
                    }
                }
            }
        }
    }

    fn upload_allocation(
        &mut self,
        data: &[u8],
        image_width: u32,
        image_height: u32,
        padding: u32,
        offset: usize,
        allocation: &Allocation,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        use wgpu::util::DeviceExt;

        let (x, y) = allocation.position();
        let Size { width, height } = allocation.size();
        let layer = allocation.layer();

        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let data_size = data.len() as u64;
        let max_buffer_size = device.limits().max_buffer_size;

        if data_size > max_buffer_size {
            log::error!(
                "Buffer size {} bytes exceeds device maximum {} bytes. \
                 Falling back to chunked upload for {}x{} allocation",
                data_size,
                max_buffer_size,
                width,
                height
            );
            self.upload_allocation_chunked(
                data,
                image_width,
                image_height,
                padding,
                offset,
                allocation,
                device,
                encoder,
            );
            return;
        }

        let buffer_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("image upload buffer"),
                contents: data,
                usage: wgpu::BufferUsages::COPY_SRC,
            })
        }));

        let buffer = match buffer_result {
            Ok(buf) => buf,
            Err(_) => {
                log::error!(
                    "Failed to create buffer for {}x{} allocation ({} bytes). \
                     Falling back to chunked upload.",
                    width,
                    height,
                    data_size
                );
                self.upload_allocation_chunked(
                    data,
                    image_width,
                    image_height,
                    padding,
                    offset,
                    allocation,
                    device,
                    encoder,
                );
                return;
            }
        };

        encoder.copy_buffer_to_texture(
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: offset as u64,
                    bytes_per_row: Some(4 * image_width + padding),
                    rows_per_image: Some(image_height),
                },
            },
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x,
                    y,
                    z: layer as u32,
                },
                aspect: wgpu::TextureAspect::default(),
            },
            extent,
        );
    }

    /// Chunked upload fallback for when a single allocation would exceed GPU buffer limits.
    /// This splits the data into smaller chunks that can each fit in a buffer.
    fn upload_allocation_chunked(
        &mut self,
        data: &[u8],
        image_width: u32,
        image_height: u32,
        padding: u32,
        offset: usize,
        allocation: &Allocation,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        use wgpu::util::DeviceExt;

        let (x, y) = allocation.position();
        let Size { width, height } = allocation.size();
        let layer = allocation.layer();

        let max_buffer_size = device.limits().max_buffer_size as usize;
        let bytes_per_row = (4 * image_width + padding) as usize;

        // Calculate how many rows we can fit in one chunk (with 20% safety margin)
        let max_rows_per_chunk = ((max_buffer_size * 4) / (5 * bytes_per_row)).max(1) as u32;

        log::warn!(
            "Chunked upload: splitting {}x{} allocation into chunks of {} rows each",
            width,
            height,
            max_rows_per_chunk
        );

        let mut current_row = 0;

        while current_row < height {
            let rows_in_chunk = (height - current_row).min(max_rows_per_chunk);
            let chunk_start = offset + (current_row as usize * bytes_per_row);
            let chunk_size = rows_in_chunk as usize * bytes_per_row;
            let chunk_data = &data[chunk_start..chunk_start + chunk_size];

            // Attempt buffer creation with panic protection
            let buffer_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("image upload buffer (chunked)"),
                    contents: chunk_data,
                    usage: wgpu::BufferUsages::COPY_SRC,
                })
            }));

            let buffer = match buffer_result {
                Ok(buf) => buf,
                Err(_) => {
                    log::error!(
                        "Failed to create chunked buffer for rows {}-{} ({} bytes). Skipping chunk.",
                        current_row,
                        current_row + rows_in_chunk,
                        chunk_size
                    );
                    current_row += rows_in_chunk;
                    continue;
                }
            };

            encoder.copy_buffer_to_texture(
                wgpu::ImageCopyBuffer {
                    buffer: &buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * image_width + padding),
                        rows_per_image: Some(rows_in_chunk),
                    },
                },
                wgpu::ImageCopyTexture {
                    texture: &self.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x,
                        y: y + current_row,
                        z: layer as u32,
                    },
                    aspect: wgpu::TextureAspect::default(),
                },
                wgpu::Extent3d {
                    width,
                    height: rows_in_chunk,
                    depth_or_array_layers: 1,
                },
            );

            current_row += rows_in_chunk;
        }
    }

    fn grow(
        &mut self,
        amount: usize,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        if amount == 0 {
            return;
        }

        let new_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("iced_wgpu::image texture atlas"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: self.layers.len() as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if color::GAMMA_CORRECTION {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            },
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let amount_to_copy = self.layers.len() - amount;

        for (i, layer) in
            self.layers.iter_mut().take(amount_to_copy).enumerate()
        {
            if layer.is_empty() {
                continue;
            }

            encoder.copy_texture_to_texture(
                wgpu::ImageCopyTexture {
                    texture: &self.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: wgpu::TextureAspect::default(),
                },
                wgpu::ImageCopyTexture {
                    texture: &new_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: wgpu::TextureAspect::default(),
                },
                wgpu::Extent3d {
                    width: SIZE,
                    height: SIZE,
                    depth_or_array_layers: 1,
                },
            );
        }

        self.texture = new_texture;
        self.texture_view =
            self.texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            });

        self.texture_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("iced_wgpu::image texture atlas bind group"),
                layout: &self.texture_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &self.texture_view,
                    ),
                }],
            });
    }
}
