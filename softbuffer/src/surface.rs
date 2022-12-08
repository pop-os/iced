use cosmic_text::{Attrs, AttrsList, BufferLine, SwashCache, SwashContent};
use iced_graphics::{Background, Primitive, alignment::{Horizontal, Vertical}};
use raqote::{DrawOptions, DrawTarget, Image, IntPoint, IntRect, PathBuilder, SolidSource, Source, Transform};
use raw_window_handle::{
    HasRawDisplayHandle,
    HasRawWindowHandle,
    RawDisplayHandle,
    RawWindowHandle
};
use softbuffer::GraphicsContext;


// Wrapper to get around lifetimes in GraphicsContext
#[derive(Debug)]
struct RawWindow {
    display_handle: RawDisplayHandle,
    window_handle: RawWindowHandle,
}

unsafe impl HasRawDisplayHandle for RawWindow {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.display_handle
    }
}

unsafe impl HasRawWindowHandle for RawWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.window_handle
    }
}

// A software rendering surface
pub struct Surface {
    context: GraphicsContext<RawWindow>,
    width: u32,
    height: u32,
    buffer: Vec<u32>,
}

impl Surface {
    pub(crate) fn new<W: HasRawWindowHandle + HasRawDisplayHandle>(window: &W) -> Self {
        let raw_window = crate::surface::RawWindow {
            display_handle: window.raw_display_handle(),
            window_handle: window.raw_window_handle(),
        };

        let context = match unsafe { GraphicsContext::new(raw_window) } {
            Ok(ok) => ok,
            Err(err) => panic!("failed to create softbuffer context: {}", err),
        };
        Surface {
            context,
            width: 0,
            height: 0,
            buffer: Vec::new(),
        }
    }

    pub(crate) fn configure(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.buffer = vec![
            0;
            self.width as usize * self.height as usize
        ];
    }

    pub(crate) fn present<Theme>(&mut self, renderer: &mut crate::Renderer<Theme>, background: iced_graphics::Color) {
        {
            let mut draw_target = DrawTarget::from_backing(
                self.width as i32,
                self.height as i32,
                self.buffer.as_mut_slice()
            );

            draw_target.clear({
                let rgba = background.into_rgba8();
                SolidSource::from_unpremultiplied_argb(
                    rgba[3],
                    rgba[0],
                    rgba[1],
                    rgba[2],
                )
            });

            let draw_options = DrawOptions::new();

            // Having at least one clip fixes some font rendering issues
            draw_target.push_clip_rect(IntRect::new(
                IntPoint::new(0, 0),
                IntPoint::new(self.width as i32, self.height as i32)
            ));

            for primitive in renderer.primitives.iter() {
                draw_primitive(&mut draw_target, &draw_options, &mut renderer.swash_cache, primitive);
            }

            draw_target.pop_clip();
        }

        self.context.set_buffer(
            &self.buffer,
            self.width as u16,
            self.height as u16
        );
    }
}

fn draw_primitive(draw_target: &mut DrawTarget<&mut [u32]>, draw_options: &DrawOptions, swash_cache: &mut SwashCache, primitive: &Primitive) {
    match primitive {
        Primitive::None => (),
        Primitive::Group { primitives } => {
            for child in primitives.iter() {
                draw_primitive(draw_target, draw_options, swash_cache, child);
            }
        },
        Primitive::Text {
            content,
            bounds,
            color,
            size,
            font,
            horizontal_alignment,
            vertical_alignment,
        } => {
            let cosmic_color = {
                let rgba8 = color.into_rgba8();
                cosmic_text::Color::rgba(
                    rgba8[0],
                    rgba8[1],
                    rgba8[2],
                    rgba8[3],
                )
            };

            //TODO: how to properly calculate line height?
            let line_height = *size as i32 * 5 / 4;

            let mut buffer_line = BufferLine::new(content, AttrsList::new(Attrs::new()));
            let buffer_width = i32::max_value(); // TODO: allow wrapping
            let layout = buffer_line.layout(&crate::renderer::FONT_SYSTEM, *size as i32, buffer_width);

            let mut line_y = match vertical_alignment {
                Vertical::Top => bounds.y as i32 + *size as i32,
                Vertical::Center => {
                    let center = (bounds.y + bounds.height / 2.0) as i32;
                    center + *size as i32/2 - line_height * layout.len() as i32 / 2
                }
                Vertical::Bottom => {
                    let bottom = (bounds.y + bounds.height) as i32;
                    bottom + *size as i32 - line_height * layout.len() as i32
                },
            };
            println!("{:?}: {:?} {:?} = {}", content, bounds, vertical_alignment, line_y);

            for layout_line in layout.iter() {
                let mut line_width = 0.0;
                for glyph in layout_line.glyphs.iter() {
                    let max_x = if glyph.rtl {
                        glyph.x - glyph.w
                    } else {
                        glyph.x + glyph.w
                    };
                    if max_x > line_width {
                        line_width = max_x;
                    }
                }

                let line_x = match horizontal_alignment {
                    Horizontal::Left => bounds.x as i32,
                    Horizontal::Center => {
                        let center = (bounds.x + bounds.width / 2.0) as i32;
                        center - line_width as i32 / 2
                    },
                    Horizontal::Right => {
                        let right = (bounds.x + bounds.width) as i32;
                        right - line_width as i32
                    }
                };

                for glyph in layout_line.glyphs.iter() {
                    let (cache_key, x_int, y_int) = (glyph.cache_key, glyph.x_int, glyph.y_int);

                    let glyph_color = match glyph.color_opt {
                        Some(some) => some,
                        None => cosmic_color,
                    };

                    if let Some(image) = swash_cache.get_image(cache_key) {
                        let x = line_x + x_int + image.placement.left;
                        let y = line_y + y_int + -image.placement.top;

                        let mut image_data = Vec::new();
                        match image.content {
                            SwashContent::Mask => {
                                let mut i = 0;
                                for _off_y in 0..image.placement.height as i32 {
                                    for _off_x in 0..image.placement.width as i32 {
                                        //TODO: blend base alpha?
                                        image_data.push(
                                            SolidSource::from_unpremultiplied_argb(
                                                image.data[i],
                                                glyph_color.r(),
                                                glyph_color.g(),
                                                glyph_color.b(),
                                            ).to_u32()
                                        );
                                        i += 1;
                                    }
                                }
                            },
                            SwashContent::Color => {
                                let mut i = 0;
                                for _off_y in 0..image.placement.height as i32 {
                                    for _off_x in 0..image.placement.width as i32 {
                                        //TODO: blend base alpha?
                                        image_data.push(
                                            SolidSource::from_unpremultiplied_argb(
                                                image.data[i + 3],
                                                image.data[i + 0],
                                                image.data[i + 1],
                                                image.data[i + 2],
                                            ).to_u32()
                                        );
                                        i += 4;
                                    }
                                }

                            },
                            SwashContent::SubpixelMask => {
                                eprintln!("Content::SubpixelMask");
                            }
                        }

                        if ! image_data.is_empty() {
                            draw_target.draw_image_at(
                                x as f32,
                                y as f32,
                                &Image {
                                    width: image.placement.width as i32,
                                    height: image.placement.height as i32,
                                    data: &image_data
                                },
                                &draw_options
                            );
                        }
                    }
                }

                line_y += line_height;
            }
        },
        Primitive::Quad {
            bounds,
            background,
            border_radius,
            border_width,
            border_color,
        } => {
            let mut pb = PathBuilder::new();

            // Move to top left corner at start of clockwise arc
            pb.move_to(bounds.x, bounds.y + border_radius);
            pb.arc(
                bounds.x + border_radius,
                bounds.y + border_radius,
                *border_radius,
                180.0f32.to_radians(),
                90.0f32.to_radians()
            );

            // Move to top right corner at start of clockwise arc
            pb.line_to(bounds.x + bounds.width - border_radius, bounds.y);
            pb.arc(
                bounds.x + bounds.width - border_radius,
                bounds.y + border_radius,
                *border_radius,
                270.0f32.to_radians(),
                90.0f32.to_radians()
            );

            // Move to bottom right corner at start of clockwise arc
            pb.line_to(bounds.x + bounds.width, bounds.y + bounds.height - border_radius);
            pb.arc(
                bounds.x + bounds.width - border_radius,
                bounds.y + bounds.height - border_radius,
                *border_radius,
                0.0f32.to_radians(),
                90.0f32.to_radians()
            );

            // Move to bottom left corner at start of clockwise arc
            pb.line_to(bounds.x + border_radius, bounds.y + bounds.height);
            pb.arc(
                bounds.x + border_radius,
                bounds.y + bounds.height - border_radius,
                *border_radius,
                90.0f32.to_radians(),
                90.0f32.to_radians()
            );

            // Close and finish path
            pb.close();
            let path = pb.finish();

            let background_source = match background {
                Background::Color(color) => {
                    let rgba = color.into_rgba8();
                    Source::Solid(SolidSource::from_unpremultiplied_argb(
                        rgba[3],
                        rgba[0],
                        rgba[1],
                        rgba[2],
                    ))
                }
            };

            draw_target.fill(
                &path,
                &background_source,
                draw_options
            );

            let border_source = {
                let rgba = border_color.into_rgba8();
                raqote::Source::Solid(
                    raqote::SolidSource::from_unpremultiplied_argb(
                        rgba[3],
                        rgba[0],
                        rgba[1],
                        rgba[2],
                    )
                )
            };

            let style = raqote::StrokeStyle {
                width: *border_width,
                ..Default::default()
            };

            draw_target.stroke(
                &path,
                &border_source,
                &style,
                draw_options
            );
        },
        Primitive::Clip {
            bounds,
            content,
        } => {
            draw_target.push_clip_rect(IntRect::new(
                IntPoint::new(
                    bounds.x as i32,
                    bounds.y as i32
                ),
                IntPoint::new(
                    (bounds.x + bounds.width) as i32,
                    (bounds.y + bounds.height) as i32
                )
            ));
            draw_primitive(draw_target, draw_options, swash_cache, &content);
            draw_target.pop_clip();
        },
        Primitive::Translate {
            translation,
            content,
        } => {
            draw_target.set_transform(&Transform::translation(
                translation.x,
                translation.y
            ));
            draw_primitive(draw_target, draw_options, swash_cache, &content);
            draw_target.set_transform(&Transform::identity());
        },
        _ => {
            eprintln!("{:?}", primitive);
        },
    }
}
