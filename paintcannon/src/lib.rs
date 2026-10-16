// NAPI's type generator must see Task implementations before their return types.
mod glyph_detection;

mod api;
mod engine;
mod event_notification;
mod frame;
mod image;
mod input;
mod input_reader;
mod layout;
mod line_break;
mod native_event;
mod paint;
mod selection;
mod style;
mod terminal;
mod text;
mod text_wrap;
mod transition;

pub use api::PaintCannon;
