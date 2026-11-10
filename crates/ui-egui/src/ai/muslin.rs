//! Check My Muslin (Help and Pattern menus): photos of a muslin (a toile, the test garment) tried
//! on, the pattern it was cut from when it is open, and a few answers about the fit go to Claude,
//! which says what doesn't fit and why and offers to alter the pattern. The check runs as an Ask
//! About Patterns conversation in its own mode ([`super::ask::begin_check`]): the answer, the
//! changes to allow and the questions after it are in that window.
//!
//! Photos are turned upright (a phone's orientation tag), made no larger than Claude looks at and
//! sent as JPEG ([`prepare`]). On the desktop they are picked in an open dialog; on the web the
//! browser's picker hands them in through the place inbox ([`take_arrivals`]). The photos show
//! the user's body: they are kept only while the window is open and go with the check alone.
//!
//! Adapted from StitchFoundry's Check My Sew-out (`crates/ui-egui/src/embroidery/sewout.rs`).

use egui::RichText;
use pf_ai::AiImage;
use pf_ai::muslin::{self, Answers};
use serde_json::{Value, json};

use crate::VectorcraftApp;
use crate::theme::Tokens;

/// The thumbnails' longest side (px).
const THUMB: u32 = 96;
/// The largest photo file read (bytes).
const MAX_FILE: usize = 60_000_000;
/// The largest photo decoded (pixels a side).
const MAX_SIDE: u32 = 20_000;

/// A photo ready to send.
pub struct Photo {
    pub name: String,
    pub image: AiImage,
    /// Its size as sent (px).
    pub size: (u32, u32),
    thumb: egui::ColorImage,
    texture: Option<egui::TextureHandle>,
}

/// The window between frames. Not saved.
#[derive(Default)]
pub struct MuslinState {
    pub open: bool,
    pub photos: Vec<Photo>,
    /// The muslin's fabric and the fabric the garment will be made in, as typed.
    pub fabric: String,
    pub final_fabric: String,
    /// How it feels ([`feels`]).
    pub feels: Vec<bool>,
    pub notes: String,
    /// Send a summary of the pattern open now with the photos.
    pub include_pattern: bool,
    /// The web's picker is open for photos: the files it hands in are photos.
    pub waiting: bool,
    pub error: Option<String>,
}

/// How a muslin can feel: (sent to Claude, shown).
fn feels() -> [(&'static str, &'static str); 8] {
    [
        ("tight across the bust or chest", tl!("Tight across the bust or chest")),
        ("tight across the back or shoulders", tl!("Tight across the back or shoulders")),
        ("tight at the waist or hips", tl!("Tight at the waist or hips")),
        ("loose or gaping somewhere", tl!("Loose or gaping somewhere")),
        ("hard to raise my arms", tl!("Hard to raise my arms")),
        ("the neckline gapes or pulls", tl!("The neckline gapes or pulls")),
        ("too long or too short", tl!("Too long or too short")),
        ("the shoulder seams don't sit on my shoulders", tl!("The shoulder seams don't sit on my shoulders")),
    ]
}

/// `ui.checkMuslin {photos?: [path]}`: opens the window, with the photos at those paths added.
pub fn open(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    if !super::available(app) {
        return Err("Check My Muslin needs a build that can reach the AI service".into());
    }
    let st = &mut app.ai.muslin;
    if !st.open && st.photos.is_empty() {
        st.include_pattern = true;
    }
    st.open = true;
    st.feels.resize(feels().len(), false);
    let paths: Vec<String> = p.get("photos").and_then(Value::as_array).into_iter().flatten().filter_map(|v| v.as_str().map(str::to_string)).collect();
    read_photos(app, paths);
    Ok(json!({ "open": true, "photos": app.ai.muslin.photos.len(), "error": app.ai.muslin.error }))
}

/// A photo's bytes as Claude is sent them: upright, no larger than it looks at, as JPEG; with its
/// thumbnail. An error says why it can't be read.
pub fn prepare(bytes: &[u8]) -> Result<(AiImage, (u32, u32), egui::ColorImage), String> {
    use image::ImageDecoder;
    if bytes.len() > MAX_FILE {
        return Err("This photo is too large: send a smaller copy.".into());
    }
    let unreadable =
        || "This photo can't be read: save or send it as a JPEG (on an iPhone: Settings › Camera › Formats › Most Compatible).".to_string();
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().map_err(|_| unreadable())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|_| unreadable())?;
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = image::DynamicImage::from_decoder(decoder).map_err(|_| unreadable())?;
    img.apply_orientation(orientation);
    let side = pf_ai::IMAGE_SIDE;
    if img.width() > side || img.height() > side {
        img = img.resize(side, side, image::imageops::FilterType::Triangle);
    }
    let rgb = img.to_rgb8();
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 85).encode_image(&rgb).map_err(|e| e.to_string())?;
    let thumb = img.thumbnail(THUMB, THUMB).to_rgba8();
    let thumb = egui::ColorImage::from_rgba_unmultiplied([thumb.width() as usize, thumb.height() as usize], thumb.as_raw());
    let image = AiImage { media_type: "image/jpeg", data_base64: vectorcraft_format::base64_encode(&jpeg) };
    Ok((image, (rgb.width(), rgb.height()), thumb))
}

/// Add the photo `bytes` (file `name`); why not, in the window. Its name is never logged.
pub fn add_photo(app: &mut VectorcraftApp, name: &str, bytes: &[u8]) {
    let st = &mut app.ai.muslin;
    if st.photos.len() >= muslin::MAX_PHOTOS {
        st.error = Some("At most 6 photos: remove one first.".into());
        return;
    }
    match prepare(bytes) {
        Ok((image, size, thumb)) => {
            st.photos.push(Photo { name: name.to_string(), image, size, thumb, texture: None });
            st.error = None;
        }
        // (the message alone: it is shown translated)
        Err(e) => st.error = Some(e),
    }
}

/// Add Photos…: picked in an open dialog (desktop), or in the browser's picker (web; they arrive
/// through [`take_arrivals`]).
fn add_photos(app: &mut VectorcraftApp) {
    if let Some(start) = app.services.place_async.as_mut() {
        start();
        app.ai.muslin.waiting = true;
        return;
    }
    let paths = crate::picks::open_many(app);
    read_photos(app, paths);
}

/// The photos at `paths` added (desktop).
fn read_photos(app: &mut VectorcraftApp, paths: Vec<String>) {
    for path in paths {
        let read = match app.services.read.as_ref() {
            Some(read) => read(&path),
            None => Err("no files here".into()),
        };
        let name = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.clone());
        match read {
            Ok(bytes) => add_photo(app, &name, &bytes),
            Err(e) => app.ai.muslin.error = Some(e),
        }
    }
}

/// Web: the files the browser's picker handed in while it was open for photos → the ones taken
/// (the rest go on to Place).
pub fn take_arrivals(app: &mut VectorcraftApp, arrived: Vec<crate::place::PlaceArrival>) -> Vec<crate::place::PlaceArrival> {
    if !app.ai.muslin.waiting {
        return arrived;
    }
    let mut rest = Vec::new();
    let mut took = false;
    for a in arrived {
        if a.drop.is_none() {
            add_photo(app, &a.name, &a.bytes);
            took = true;
        } else {
            rest.push(a);
        }
    }
    if took {
        app.ai.muslin.waiting = false;
    }
    rest
}

/// Check (the window's button, `ui.checkMuslin {check: true}`): the photos and the answers go
/// to Claude, and the Ask window shows the answer. The photos are let go of once sent.
pub fn check(app: &mut VectorcraftApp) -> Result<(), String> {
    let with_pattern = app.ai.muslin.include_pattern && app.session.active().is_some();
    let st = &app.ai.muslin;
    let feels: Vec<String> = feels().iter().zip(&st.feels).filter(|(_, on)| **on).map(|((en, _), _)| en.to_string()).collect();
    let answers = Answers { fabric: st.fabric.clone(), final_fabric: st.final_fabric.clone(), feels, notes: st.notes.clone() };
    let photos: Vec<AiImage> = st.photos.iter().map(|p| p.image.clone()).collect();
    let n = photos.len();
    let summary = if with_pattern { super::ask::pattern_summary(app) } else { None };
    let message = match muslin::first_message(&photos, summary.as_deref(), &answers) {
        Ok(m) => m,
        Err(e) => {
            let m = e.to_string();
            app.ai.muslin.error = Some(m.clone());
            return Err(m);
        }
    };
    let shown = crate::i18n::fmt(tl!("My muslin ({n} photos)"), &[("n", &n.to_string())]);
    super::ask::begin_check(app, message, shown)?;
    let st = &mut app.ai.muslin;
    st.open = false;
    st.photos.clear();
    Ok(())
}

/// The window, while open.
pub fn window(app: &mut VectorcraftApp, ctx: &egui::Context) {
    if !app.ai.muslin.open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    let mut add = false;
    let mut remove = None;
    let mut go = false;
    let has_doc = app.session.active().is_some();
    egui::Window::new(tl!("Check My Muslin"))
        .id(egui::Id::new("check-my-muslin"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_pos(ctx.content_rect().center() - egui::vec2(250.0, 300.0))
        .show(ctx, |ui| {
            ui.set_width(500.0);
            let st = &mut app.ai.muslin;
            ui.label(RichText::new(tl!("Photos of your muslin, tried on")).strong().size(15.0));
            ui.label(
                RichText::new(tl!("The front, the side and the back, standing straight with your arms down, and close-ups of any folds or pulling."))
                    .size(12.0)
                    .color(t.text_dim),
            );
            ui.horizontal_wrapped(|ui| {
                for (i, p) in st.photos.iter_mut().enumerate() {
                    ui.vertical(|ui| {
                        let tex = p.texture.get_or_insert_with(|| ui.ctx().load_texture(format!("muslin-{i}"), p.thumb.clone(), Default::default()));
                        ui.image((tex.id(), tex.size_vec2()));
                        if ui.small_button(tl!("Remove")).clicked() {
                            remove = Some(i);
                        }
                    });
                }
            });
            ui.horizontal(|ui| {
                let full = st.photos.len() >= muslin::MAX_PHOTOS;
                if ui.add_enabled(!full, egui::Button::new(tl!("Add Photos…"))).clicked() {
                    add = true;
                }
                if st.waiting {
                    ui.label(RichText::new(tl!("Choose the photos in the browser's window…")).size(12.0).color(t.text_dim));
                }
            });
            ui.add_space(6.0);
            ui.label(RichText::new(tl!("About the fit")).strong());
            egui::Grid::new("muslin-answers").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label(tl!("Muslin fabric"));
                ui.add(egui::TextEdit::singleline(&mut st.fabric).desired_width(260.0).hint_text(tl!("e.g. calico, an old sheet")));
                ui.end_row();
                ui.label(tl!("Final fabric"));
                ui.add(egui::TextEdit::singleline(&mut st.final_fabric).desired_width(260.0).hint_text(tl!("e.g. cotton poplin, jersey")));
                ui.end_row();
            });
            ui.label(tl!("How it feels:"));
            st.feels.resize(feels().len(), false);
            for ((_, label), on) in feels().iter().zip(st.feels.iter_mut()) {
                ui.checkbox(on, *label);
            }
            ui.add(
                egui::TextEdit::multiline(&mut st.notes)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .hint_text(tl!("Anything else? For example: folds below the back neck")),
            );
            ui.add_enabled_ui(has_doc, |ui| {
                ui.checkbox(&mut st.include_pattern, tl!("Include the pattern it was cut from (open now), so it can be altered"));
            });
            ui.label(
                RichText::new(tl!("The photos go to Claude for this check only; PatternFoundry doesn't keep them.")).size(11.0).color(t.text_dim),
            );
            if let Some(e) = &st.error {
                ui.label(RichText::new(crate::i18n::msg(e)).color(t.error));
            }
            ui.add_space(4.0);
            let ready = !st.photos.is_empty() && !app.ai.ask.asking();
            if ui.add_enabled_ui(ready, |ui| crate::widgets::primary_button(ui, tl!("Check My Muslin"))).inner.clicked() {
                go = true;
            }
            super::account::paid_line(app, ui, &t);
            ui.label(
                RichText::new(tl!("Claude looks at the photos. It can be wrong: sew the altered part again in muslin before cutting good fabric."))
                    .size(11.0)
                    .color(t.text_dim),
            );
        });
    if let Some(i) = remove
        && i < app.ai.muslin.photos.len()
    {
        app.ai.muslin.photos.remove(i);
    }
    if add {
        crate::picks::button(app, add_photos);
    }
    if go {
        // (a failure is shown in the window it happened in)
        let _ = check(app);
    }
    if !open {
        let st = &mut app.ai.muslin;
        st.open = false;
        st.waiting = false;
        // The photos show the user: not kept once the window closes.
        st.photos.clear();
    }
}
