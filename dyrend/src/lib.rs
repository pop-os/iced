//! A [`swbuf`] renderer for [`iced_native`].
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/iced-rs/iced/9ab6923e943f784985e9ef9ca28b10278297225d/docs/logo.svg"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod renderer;
pub use self::renderer::Renderer;

pub mod settings;
pub use self::settings::Settings;

pub mod window;
