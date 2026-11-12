//! Designs placed on a piece (PF-12): an embroidery design or a screen-print artwork laid on a
//! pattern piece at its true size, to see how big it is and where it goes before cutting.
//!
//! The design's own size comes from its file where the format says it (a DST's header, a PNG's
//! pixels and declared resolution, an SVG's width and height or view box) or from `widthMm` and
//! `heightMm`. The placement keeps that size, a scale, a turn and the place of its centre on the
//! piece ([`pf_core::Placement`]); the outline and crosshair are drawn from it as a generated part
//! of the piece.
use super::*;
use pf_core::placement::{MM_PER_IN, Placement, PlacementKind};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "piece.addPlacement",
            "Place Design",
            [],
            None,
            "{id?, path | name + dataBase64 (the design's file: an embroidery file (DST, PES…) or a ScreenFoundry artwork or separation, PNG or SVG), kind?: \"embroidery\" | \"screen\" (default: from the file's extension), title?, widthMm?/heightMm? (the design's own size, mm or with a unit; needed where the file doesn't say it, as PES; one of them keeps the file's proportions), point?: [x, y] (the design's centre in document points; default the middle of the piece), scale?: 1, rotation?: degrees clockwise 0, snap?: \"grain\" | \"fold\", hoop?: [width, height] mm | \"100x100\" | \"4x4 in\" (the embroidery hoop, drawn round the design and turned with it)} the design is laid on the piece at its true size, outlined with a crosshair at its centre on the pattern → piece.info with placed: {index, name, kind, app (StitchFoundry | ScreenFoundry), widthMm, heightMm, size, sizeIn, note, hoop, fitsHoop, warnings (a design larger than its hoop, or design or hoop past the cutting line)}",
            has_doc,
            add
        ),
        cmd!(
            "piece.setPlacement",
            "Move or Size Design",
            [],
            None,
            "{id?, index? (the design's place in piece.info's placements; default the last), point?: [x, y] (the centre, document points), moveMm?: [dx, dy] (move by mm), scale?, widthMm? | heightMm? (size to this true width or height, proportions kept), rotation?: degrees clockwise, snap?: \"grain\" (the centre onto the grainline) | \"fold\" (onto the fold line, the centre front or back of a piece cut on the fold), hoop?: as piece.addPlacement (null: none), title?} → piece.info with placed",
            has_doc,
            set
        ),
        cmd!(
            query "piece.placementLink",
            "Design File",
            [],
            None,
            "{id?, index? (default the last)} the placed design's own file and the Foundry app that edits it, for \"Open in StitchFoundry\" / \"Open in ScreenFoundry\" → {index, name, kind, app (StitchFoundry for embroidery, ScreenFoundry for screen-print artwork), path, openable: bool (a regular file on this computer, an embroidery or PNG/SVG file; false on the web and for a design placed from bytes), reason (why not)}; the host hands `path` to the system's default app for it",
            has_doc,
            link
        ),
        cmd!(
            "piece.removePlacement",
            "Remove Design",
            [],
            None,
            "{id?, index? (default the last)} → piece.info",
            has_doc,
            remove
        ),
    ]
}

/// A number param, `None` when absent; refused when it isn't a finite number.
fn number(p: &Value, key: &str, cmd: &str) -> Result<Option<f64>> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_f64().filter(|v| v.is_finite()).map(Some).ok_or_else(|| bad(cmd, format!("{key}: a number"))),
    }
}

/// A length param in mm (a number, or a length with its unit), `None` when absent.
fn length(p: &Value, key: &str, cmd: &str) -> Result<Option<f64>> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => mm_param(v).filter(|v| *v > 0.0).map(Some).ok_or_else(|| bad(cmd, format!("{key}: a size in mm, or a length with its unit (\"3 in\")"))),
    }
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// How a placement reads in a command's result.
fn placed_json(index: usize, pl: &Placement) -> Value {
    let (w, h) = pl.size_mm();
    json!({
        "index": index,
        "name": pl.name,
        "kind": pl.kind,
        "app": pl.kind.app(),
        "widthMm": round3(w),
        "heightMm": round3(h),
        "size": pl.size_text(false),
        "sizeIn": pl.size_text(true),
        "note": pl.note(),
    })
}

/// What is wrong with how `pl` sits on piece `id`, for the sewist.
fn warnings(s: &Session, id: NodeId, pl: &Placement) -> Vec<&'static str> {
    let on_piece = |c: [Pt; 4]| c.iter().all(|q| inside_cut(s, id, *q));
    let mut w = Vec::new();
    if pl.fits_hoop() == Some(false) {
        w.push("the design is larger than the hoop");
    }
    if !on_piece(pl.corners()) {
        w.push("the design reaches past the piece's cutting line");
    }
    if pl.hoop_corners().is_some_and(|c| !on_piece(c)) {
        w.push("the hoop reaches past the piece's cutting line");
    }
    w
}

/// `info` with the placement `index` of piece `id` as `placed`.
fn with_placed(s: &Session, id: NodeId, mut info: Value, index: usize) -> Value {
    let piece = piece_of(s, id);
    if let (Some(o), Some(pl)) = (info.as_object_mut(), piece.as_ref().and_then(|pc| pc.placements.get(index))) {
        let mut placed = placed_json(index, pl);
        placed["hoop"] = json!(pl.hoop);
        placed["fitsHoop"] = json!(pl.fits_hoop());
        placed["warnings"] = json!(warnings(s, id, pl));
        o.insert("placed".into(), placed);
    }
    info
}

/// The `hoop` param: `[width, height]` (mm, or lengths with a unit) or text such as `"100x100"` or
/// `"4x4 in"`; null for no hoop. `None` when the param is absent.
fn hoop_param(p: &Value, cmd: &str) -> Result<Option<Option<[f64; 2]>>> {
    let msg = "hoop: [width, height] in mm, or text such as \"100x100\" or \"4x4 in\" (null: no hoop)";
    match p.get("hoop") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(t)) => pf_core::placement::parse_hoop(t).map(|h| Some(Some(h))).ok_or_else(|| bad(cmd, msg)),
        Some(Value::Array(a)) if a.len() == 2 => match (mm_param(&a[0]), mm_param(&a[1])) {
            (Some(w), Some(h)) => Ok(Some(Some([w, h]))),
            _ => Err(bad(cmd, msg)),
        },
        Some(_) => Err(bad(cmd, msg)),
    }
}

fn kind_of_extension(name: &str) -> Option<PlacementKind> {
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    match ext.as_str() {
        "dst" | "pes" | "pec" | "exp" | "jef" | "vp3" | "vip" | "hus" | "xxx" | "sew" | "pcs" | "u01" | "tbf" | "10o" => Some(PlacementKind::Embroidery),
        "png" | "svg" => Some(PlacementKind::Screen),
        _ => None,
    }
}

/// The design's own size in mm, where the file says it.
fn design_size(name: &str, bytes: &[u8]) -> Option<(f64, f64)> {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        png_size(bytes)
    } else if ext == "dst" {
        dst_size(bytes)
    } else if ext == "svg" {
        svg_size(bytes)
    } else {
        None
    }
}

/// A Tajima DST's size from its header: the extents `+X:`, `-X:`, `+Y:` and `-Y:` are in tenths of
/// a millimetre from the start point.
fn dst_size(b: &[u8]) -> Option<(f64, f64)> {
    let head = String::from_utf8_lossy(b.get(..b.len().min(512))?).into_owned();
    let field = |key: &str| head.split(['\r', '\n']).find_map(|l| l.strip_prefix(key)).and_then(|v| v.trim().trim_matches('\0').parse::<f64>().ok());
    let (px, nx, py, ny) = (field("+X:")?, field("-X:")?, field("+Y:")?, field("-Y:")?);
    let (w, h) = ((px.abs() + nx.abs()) / 10.0, (py.abs() + ny.abs()) / 10.0);
    (w > 0.0 && h > 0.0).then_some((w, h))
}

/// A PNG's size from its pixels and the resolution it declares (72 ppi when it declares none, as
/// File › Place sizes an image).
fn png_size(b: &[u8]) -> Option<(f64, f64)> {
    let px = |at: usize| b.get(at..at + 4).and_then(|s| <[u8; 4]>::try_from(s).ok()).map(u32::from_be_bytes);
    let (w, h) = (px(16)?, px(20)?);
    let (xr, yr) = super::super::fileio::ppi::resolution(b).unwrap_or((72.0, 72.0));
    let (w, h) = (f64::from(w) / xr * MM_PER_IN, f64::from(h) / yr * MM_PER_IN);
    (w > 0.0 && h > 0.0).then_some((w, h))
}

/// An SVG's size from its root element's width and height, else its view box (CSS pixels, 96 to
/// the inch, when it gives no unit).
fn svg_size(b: &[u8]) -> Option<(f64, f64)> {
    let text = String::from_utf8_lossy(&b[..b.len().min(65_536)]).into_owned();
    let start = text.find("<svg")?;
    let tag = &text[start..];
    let tag = &tag[..tag.find('>')?];
    let attr = |name: &str| -> Option<String> {
        let mut from = 0;
        while let Some(i) = tag[from..].find(name) {
            let at = from + i;
            let after = tag[at + name.len()..].trim_start();
            if tag[..at].ends_with(char::is_whitespace)
                && let Some(rest) = after.strip_prefix('=')
            {
                let rest = rest.trim_start();
                let q = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
                return rest[1..].split(q).next().map(str::to_string);
            }
            from = at + name.len();
        }
        None
    };
    let sized = attr("width").and_then(|w| Some((css_mm(&w)?, css_mm(&attr("height")?)?)));
    sized.or_else(|| {
        let v: Vec<f64> = attr("viewBox")?.split([' ', ',', '\t', '\r', '\n']).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect();
        let [_, _, w, h] = v[..] else { return None };
        let px = MM_PER_IN / 96.0;
        (w > 0.0 && h > 0.0).then_some((w * px, h * px))
    })
}

/// A CSS length in mm: a number with `mm`, `cm`, `in`, `pt`, `pc` or `px` (or none: pixels).
fn css_mm(s: &str) -> Option<f64> {
    let s = s.trim();
    let unit_at = s.rfind(|c: char| !c.is_ascii_alphabetic() && c != '%').map_or(0, |i| i + 1);
    let (num, unit) = s.split_at(unit_at);
    let v: f64 = num.trim().parse().ok()?;
    let per = match unit.to_ascii_lowercase().as_str() {
        "" | "px" => MM_PER_IN / 96.0,
        "mm" => 1.0,
        "cm" => 10.0,
        "in" => MM_PER_IN,
        "pt" => MM_PER_IN / 72.0,
        "pc" => MM_PER_IN / 6.0,
        _ => return None,
    };
    let mm = v * per;
    (mm.is_finite() && mm > 0.0).then_some(mm)
}

/// The middle of piece `id`'s seam line's bounds, in document points.
fn centre_of(s: &Session, id: NodeId) -> Option<Pt> {
    let st = s.doc().ok()?;
    let seam = seam_of(&st.doc, id)?;
    let NodeKind::Path { path, .. } = &seam.kind else { return None };
    let (outline, _) = outline_of(path)?;
    let first = *outline.points.first()?;
    let (lo, hi) = outline.points.iter().fold((first, first), |(lo, hi), q| (Pt::new(lo.x.min(q.x), lo.y.min(q.y)), Pt::new(hi.x.max(q.x), hi.y.max(q.y))));
    Some((lo + hi) * 0.5)
}

/// The line a design snaps onto: the grainline (the piece's, else the default one) or the fold line.
fn snap_line(s: &Session, id: NodeId, snap: &str, cmd: &str) -> Result<(Pt, Pt)> {
    let st = s.doc()?;
    let piece = st.doc.node(id).and_then(piece_on).ok_or_else(|| bad(cmd, "not a pattern piece"))?;
    let seam = seam_of(&st.doc, id).ok_or_else(|| bad(cmd, "the piece has no seam line"))?;
    let NodeKind::Path { path, .. } = &seam.kind else { return Err(bad(cmd, "the seam line is not a path")) };
    match snap {
        "grain" => {
            let (outline, _) = outline_of(path).ok_or_else(|| bad(cmd, "the seam line is not a closed path"))?;
            let g = piece.grainline.or_else(|| pf_core::marks::default_grainline(&outline)).ok_or_else(|| bad(cmd, "the piece has no grainline"))?;
            Ok((g.from, g.to))
        }
        "fold" => {
            let e = piece.fold.ok_or_else(|| bad(cmd, "the piece is not cut on the fold (piece.set fold)"))?;
            let sp = path.subpaths.iter().find(|sp| sp.closed).filter(|sp| e < sp.segment_count()).ok_or_else(|| bad(cmd, "the fold edge is gone"))?;
            let seg = sp.segment(e);
            Ok((Pt::new(seg.p0.x, seg.p0.y), Pt::new(seg.p3.x, seg.p3.y)))
        }
        _ => Err(bad(cmd, "snap: \"grain\" or \"fold\"")),
    }
}

/// `q` moved along the perpendicular onto the line through `a` and `b`.
fn project(a: Pt, b: Pt, q: Pt) -> Option<Pt> {
    let d = (b - a).unit()?;
    Some(a + d * (q - a).dot(d))
}

fn index_param(p: &Value, piece: Option<&Piece>, cmd: &str) -> Result<usize> {
    let n = piece.map_or(0, |pc| pc.placements.len());
    let index = match p.get("index") {
        None | Some(Value::Null) => n.checked_sub(1),
        Some(v) => v.as_u64().and_then(|i| usize::try_from(i).ok()),
    };
    index.filter(|i| *i < n).ok_or_else(|| bad(cmd, "index: the design's place in piece.info's placements, from 0 (the piece has no such design)"))
}

fn piece_of(s: &Session, id: NodeId) -> Option<Piece> {
    s.doc().ok()?.doc.node(id).and_then(piece_on).cloned()
}

fn add(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.addPlacement";
    let id = one_piece(s, p, CMD)?;
    let src = super::super::fileio::source(p, CMD)?;
    let kind = match str_param(p, "kind") {
        Some("embroidery") => PlacementKind::Embroidery,
        Some("screen") => PlacementKind::Screen,
        Some(_) => return Err(bad(CMD, "kind: \"embroidery\" or \"screen\"")),
        None => kind_of_extension(src.name).ok_or_else(|| bad(CMD, "kind: \"embroidery\" or \"screen\" (the file's extension doesn't say)"))?,
    };
    let file = design_size(src.name, &src.bytes);
    let (w, h) = match (length(p, "widthMm", CMD)?, length(p, "heightMm", CMD)?, file) {
        (Some(w), Some(h), _) => (w, h),
        (Some(w), None, Some((fw, fh))) => (w, w * fh / fw),
        (None, Some(h), Some((fw, fh))) => (h * fw / fh, h),
        (None, None, Some(size)) => size,
        _ => {
            return Err(bad(
                CMD,
                "the file doesn't say how big the design is (PES files and many others don't): give widthMm and heightMm, in mm or with a unit",
            ));
        }
    };
    let title = str_param(p, "title").map(str::trim).filter(|t| !t.is_empty()).map(str::to_string).unwrap_or_else(|| {
        let name = src.name.rsplit(['/', '\\']).next().unwrap_or(src.name);
        name.rsplit_once('.').map_or(name, |(stem, _)| stem).to_string()
    });
    let source = src.path.map_or_else(|| src.name.to_string(), super::super::fileio::absolute_path);
    let at = match p.get("point") {
        None | Some(Value::Null) => centre_of(s, id).ok_or_else(|| bad(CMD, "the piece has no closed seam line"))?,
        Some(v) => point_of(v).ok_or_else(|| bad(CMD, "point: [x, y] in document points"))?,
    };
    let mut pl = Placement::new(kind, &pf_core::piece::check_name(&title).map_err(|e| err(CMD, e))?, &source, w, h, at);
    if let Some(k) = number(p, "scale", CMD)? {
        pl.scale = k;
    }
    if let Some(r) = number(p, "rotation", CMD)? {
        pl.rotation = r.rem_euclid(360.0);
    }
    if let Some(snap) = str_param(p, "snap") {
        let (a, b) = snap_line(s, id, snap, CMD)?;
        pl.at = project(a, b, pl.at).unwrap_or(pl.at);
    }
    if let Some(hoop) = hoop_param(p, CMD)? {
        pl.hoop = hoop;
    }
    let index = piece_of(s, id).map_or(0, |pc| pc.placements.len());
    let info = change(s, id, "Place Design", CMD, |piece| piece.add_placement(pl).map_err(|e| err(CMD, e)))?;
    Ok(with_placed(s, id, info, index))
}

fn set(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.setPlacement";
    let id = one_piece(s, p, CMD)?;
    let index = index_param(p, piece_of(s, id).as_ref(), CMD)?;
    let at = match p.get("point") {
        None | Some(Value::Null) => None,
        Some(v) => Some(point_of(v).ok_or_else(|| bad(CMD, "point: [x, y] in document points"))?),
    };
    let moved = match p.get("moveMm") {
        None | Some(Value::Null) => None,
        Some(v) => Some(finite_numbers::<2>(v).ok_or_else(|| bad(CMD, "moveMm: [dx, dy] in mm"))?),
    };
    let (scale, rotation) = (number(p, "scale", CMD)?, number(p, "rotation", CMD)?);
    let (width, height) = (length(p, "widthMm", CMD)?, length(p, "heightMm", CMD)?);
    let line = match str_param(p, "snap") {
        Some(snap) => Some(snap_line(s, id, snap, CMD)?),
        None => None,
    };
    let title = str_param(p, "title").map(|t| pf_core::piece::check_name(t).map_err(|e| err(CMD, e))).transpose()?;
    let hoop = hoop_param(p, CMD)?;
    let info = change(s, id, "Move or Size Design", CMD, |piece| {
        let pl = piece.placements.get_mut(index).ok_or_else(|| bad(CMD, "no design with that index"))?;
        if let Some(t) = title {
            pl.name = t;
        }
        if let Some(q) = at {
            pl.at = q;
        }
        if let Some([dx, dy]) = moved {
            pl.at = pl.at + Pt::new(mm_to_pt(dx), mm_to_pt(dy));
        }
        if let Some(k) = scale {
            pl.scale = k;
        }
        if let Some(w) = width {
            pl.set_width_mm(w);
        }
        if let Some(h) = height {
            pl.set_height_mm(h);
        }
        if let Some(r) = rotation {
            pl.rotation = r.rem_euclid(360.0);
        }
        if let Some(h) = hoop {
            pl.hoop = h;
        }
        if let Some((a, b)) = line {
            pl.at = project(a, b, pl.at).unwrap_or(pl.at);
        }
        if pl.is_valid() { Ok(()) } else { Err(err(CMD, pf_core::piece::PieceError::Placement)) }
    })?;
    Ok(with_placed(s, id, info, index))
}

/// Whether the design file at `path` may be handed to the system's default app: an absolute path to
/// a regular file (after following links) that is an embroidery or screen-artwork file by its
/// extension and by the extension of the file it links to; never a folder, script or shortcut.
#[cfg(not(target_arch = "wasm32"))]
fn openable(path: &str) -> std::result::Result<(), String> {
    if !std::path::Path::new(path).is_absolute() {
        return Err("the design was placed from a file's bytes, so there is no file on this computer to open".into());
    }
    crate::file_access::check_read(path)?;
    let real = std::fs::canonicalize(path).map_err(|e| format!("can't find the file: {e}"))?;
    if !real.is_file() {
        return Err("it is not a file".into());
    }
    if kind_of_extension(path).is_none() || kind_of_extension(&real.to_string_lossy()).is_none() {
        return Err("only embroidery files and PNG or SVG artwork are opened".into());
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn openable(_: &str) -> std::result::Result<(), String> {
    Err("the web version can't open files on your computer".into())
}

fn link(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.placementLink";
    let id = one_piece(s, p, CMD)?;
    let piece = piece_of(s, id);
    let index = index_param(p, piece.as_ref(), CMD)?;
    let pl = piece.as_ref().and_then(|pc| pc.placements.get(index)).ok_or_else(|| bad(CMD, "no design with that index"))?;
    let check = openable(&pl.source);
    Ok(json!({
        "index": index,
        "name": pl.name,
        "kind": pl.kind,
        "app": pl.kind.app(),
        "path": pl.source,
        "openable": check.is_ok(),
        "reason": check.err(),
    }))
}

fn remove(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.removePlacement";
    let id = one_piece(s, p, CMD)?;
    let index = index_param(p, piece_of(s, id).as_ref(), CMD)?;
    change(s, id, "Remove Design", CMD, |piece| {
        piece.placements.remove(index);
        Ok(())
    })
}
