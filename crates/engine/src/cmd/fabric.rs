//! Fabric photos (PF-10.16): an image placed in the document marked as a photo of a fabric, its
//! width on the fabric given, so the draped garment (`drape.make`) can be seen made up in it.

use super::*;
use pf_core::{FabricPhoto, PatternTag, pt_to_mm};
use serde_json::{Value, json};
use vectorcraft_doc::{Document, NodeKind};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "fabric.set",
            "Use as Fabric",
            [],
            None,
            "{id? (default: the selected image), name? (the fabric, as pieces name it, piece.set fabric; default: none, the main fabric), width?: mm (how wide the photo is on the fabric, 10 to 5000; default: as wide as the image is placed), turns?: 0..3 (quarter turns clockwise that bring the fabric's lengthwise grain, its selvedge's direction, upright in the photo), even?: bool (even out the light across the photo, brighter near a window; default false), seamless?: bool (make the photo repeat without a seam, cross-fading its edges; default false); each left out keeps what the photo had} mark an image as a photo of a fabric: drape.make draws the draped garment in it too, at true size, each piece in the photo of its fabric (else the first photo), the photo repeating across the piece along its grainline → {id, name, width, turns, even, seamless}",
            has_selection,
            set
        ),
        cmd!(
            "fabric.clear",
            "Stop Using as Fabric",
            [],
            None,
            "{id? (default: the selected image)} the image is no longer a fabric photo → {id}",
            has_selection,
            clear
        ),
        cmd!(query "fabric.info", "Fabric Photo Info", [], None, "{id? (default: the selected image)} what Use as Fabric… starts from → {id, fabric (is it a fabric photo now), name, width: mm (the photo's width on the fabric: its own, else as wide as the image is placed), turns, even, seamless}", has_doc, photo_info),
        cmd!(query "fabric.list", "Fabric Photos", [], None, "{} the document's fabric photos, in paint order → {fabrics: [{id, name, width, turns, even, seamless}]}", has_doc, list),
    ]
}

fn info(id: NodeId, f: &FabricPhoto) -> Value {
    json!({"id": id.0, "name": f.name, "width": f.width, "turns": f.turns, "even": f.even, "seamless": f.seamless})
}

fn set(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "fabric.set";
    let st = s.doc()?;
    let id = super::links::image_param(st, p, C)?;
    let old = st.doc.node(id).and_then(|n| n.extras().pattern.as_deref()).and_then(PatternTag::fabric).cloned();
    let width = match p.get("width") {
        None | Some(Value::Null) => None,
        Some(Value::String(t)) if t.trim().is_empty() => None,
        Some(v) => Some(
            super::piece::mm_param(v)
                .filter(|w| pf_core::piece::FABRIC_PHOTO_WIDTH.contains(w))
                .ok_or_else(|| bad(C, "width: the photo's width on the fabric, 10 to 5000 mm"))?,
        ),
    };
    let placed = st.doc.bounds_of(&[id], false).map(|b| pt_to_mm(b.width()));
    let width = width
        .or(old.as_ref().map(|f| f.width))
        .or(placed.filter(|w| pf_core::piece::FABRIC_PHOTO_WIDTH.contains(w)))
        .ok_or_else(|| bad(C, "width: the photo's width on the fabric, 10 to 5000 mm"))?;
    let turns = match p.get("turns") {
        None | Some(Value::Null) => old.as_ref().map_or(0, |f| f.turns),
        Some(v) => v.as_u64().filter(|t| *t <= 3).ok_or_else(|| bad(C, "turns: 0, 1, 2 or 3"))? as u8,
    };
    let flag = |key: &str, was: bool, wrong: &'static str| match p.get(key) {
        None | Some(Value::Null) => Ok(was),
        Some(v) => v.as_bool().ok_or_else(|| bad(C, wrong)),
    };
    let even = flag("even", old.as_ref().is_some_and(|f| f.even), "even: true or false")?;
    let seamless = flag("seamless", old.as_ref().is_some_and(|f| f.seamless), "seamless: true or false")?;
    let name = match str_param(p, "name") {
        Some(n) => n.trim().chars().take(pf_core::piece::MAX_NAME).collect(),
        None => old.map(|f| f.name).unwrap_or_default(),
    };
    let fabric = FabricPhoto { name, width, turns, even, seamless };
    let out = info(id, &fabric);
    s.edit("Use as Fabric", |d, _| {
        if let Some(n) = d.node_mut(id) {
            n.extras_mut().pattern = Some(Box::new(PatternTag::Fabric(fabric)));
        }
        Ok(())
    })?;
    Ok(out)
}

fn clear(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "fabric.clear";
    let st = s.doc()?;
    let id = super::links::image_param(st, p, C)?;
    if photo(&st.doc, id).is_none() {
        return Err(bad(C, "the image is not a fabric photo (Pattern › Use as Fabric…)"));
    }
    s.edit("Stop Using as Fabric", |d, _| {
        if let Some(n) = d.node_mut(id) {
            n.extras_mut().pattern = None;
        }
        Ok(())
    })?;
    Ok(json!({"id": id.0}))
}

fn photo_info(s: &mut Session, p: &Value) -> Result<Value> {
    let st = s.doc()?;
    let id = super::links::image_param(st, p, "fabric.info")?;
    let old = photo(&st.doc, id).cloned();
    let placed = st.doc.bounds_of(&[id], false).map(|b| pt_to_mm(b.width())).filter(|w| pf_core::piece::FABRIC_PHOTO_WIDTH.contains(w));
    let f = old.clone().unwrap_or(FabricPhoto { name: String::new(), width: placed.unwrap_or(1000.0), turns: 0, even: false, seamless: false });
    let mut out = info(id, &f);
    out["fabric"] = json!(old.is_some());
    Ok(out)
}

fn list(s: &mut Session, _p: &Value) -> Result<Value> {
    let fabrics: Vec<Value> = photos(&s.doc()?.doc).iter().map(|(id, f, _)| info(*id, f)).collect();
    Ok(json!({"fabrics": fabrics}))
}

/// Node `id`'s fabric photo, if it is one.
fn photo(doc: &Document, id: NodeId) -> Option<&FabricPhoto> {
    let n = doc.node(id)?;
    matches!(n.kind, NodeKind::Image(_)).then_some(())?;
    n.extras().pattern.as_deref()?.fabric()
}

/// The document's fabric photos, in paint order: each image's id, its fabric and its image key.
pub(crate) fn photos(doc: &Document) -> Vec<(NodeId, FabricPhoto, String)> {
    let mut out = Vec::new();
    doc.walk(|n| {
        if let (NodeKind::Image(im), Some(f)) = (&n.kind, n.extras().pattern.as_deref().and_then(PatternTag::fabric)) {
            out.push((n.id, f.clone(), im.key.clone()));
        }
    });
    out
}

/// The most pixels a fabric photo keeps across or down: finer than the drape view shows.
const MAX_SIDE: u32 = 2048;

/// Fabric photo `f`'s picture (image `key` of `doc`), turned upright, about `mm` a pixel (or
/// coarser where the photo is), for the drape view; `None` when it can't be read.
pub(crate) fn swatch(doc: &Document, f: &FabricPhoto, key: &str, mm: f64) -> Option<pf_core::view3d::Swatch> {
    let blob = doc.images.get(key)?;
    let img = image::load_from_memory(&blob.bytes).ok()?.to_rgb8();
    let img = match f.turns % 4 {
        1 => image::imageops::rotate90(&img),
        2 => image::imageops::rotate180(&img),
        3 => image::imageops::rotate270(&img),
        _ => img,
    };
    if img.width() == 0 || img.height() == 0 || f.width.is_nan() || f.width <= 0.0 {
        return None;
    }
    // Averaged down to the view's pixel, so a fine weave doesn't sparkle.
    let photo_mm = f.width / img.width() as f64;
    let k = (photo_mm / mm).min(1.0).max(1.0 / MAX_SIDE as f64);
    let (w, h) = (((img.width() as f64 * k).round() as u32).clamp(1, MAX_SIDE), ((img.height() as f64 * k).round() as u32).clamp(1, MAX_SIDE));
    let img = if (w, h) == img.dimensions() { img } else { image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle) };
    let mut out = pf_core::view3d::Swatch { width: w, height: h, rgb: img.into_raw(), mm: f.width / w as f64 };
    if f.even {
        out.even_lighting();
    }
    if f.seamless {
        out.make_seamless();
    }
    Some(out)
}
