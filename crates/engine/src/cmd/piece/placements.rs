//! Designs placed on a pattern piece (PF-12.1, `piece.addPlacement` and friends): an embroidery or
//! a screen print at its true size, moved, scaled and turned on the piece. The model is
//! [`pf_core::placement`]; these commands only check the params and change the piece in one undo
//! step.

use super::*;
use pf_core::placement::{DesignKind, Hoop, Placement};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "piece.addPlacement",
            "Place Design",
            [],
            None,
            "{id?, name, widthMm, heightMm (the design's own size: mm, or a length with its unit), at: [x, y] (document points, the design's centre), kind?: \"embroidery\" (default) | \"screen\", file?, scale?: 0.01..100, rotation?: degrees, hoop?: [width, height] (mm; the hoop laid round the design)} place a design on the piece at its true size → piece.info",
            has_doc,
            add_placement
        ),
        cmd!(
            "piece.setPlacement",
            "Move Placed Design",
            [],
            None,
            "{id?, index: the design's place in piece.info's placements, name?, at?: [x, y], scale?, width? | height? (mm: scale to this true size, keeping the shape), rotation?: degrees, hoop?: [width, height] | null} → piece.info",
            has_doc,
            set_placement
        ),
        cmd!(
            "piece.removePlacement",
            "Remove Placed Design",
            [],
            None,
            "{id?, index: the design's place in piece.info's placements} → piece.info",
            has_doc,
            remove_placement
        ),
    ]
}

fn hoop_param(v: &Value, cmd: &str) -> Result<Hoop> {
    let wh = v.as_array().filter(|a| a.len() == 2).and_then(|a| Some((mm_param(&a[0])?, mm_param(&a[1])?)));
    let (w, h) = wh.ok_or_else(|| bad(cmd, "hoop is [width, height] in mm"))?;
    Hoop::new(w, h).map_err(|e| err(cmd, e))
}

fn size_param(p: &Value, key: &str, cmd: &str) -> Result<f64> {
    p.get(key).and_then(mm_param).ok_or_else(|| bad(cmd, format!("{key}: the design's size in mm")))
}

fn number(p: &Value, key: &str, cmd: &str) -> Result<Option<f64>> {
    match p.get(key) {
        None => Ok(None),
        Some(v) => v.as_f64().filter(|n| n.is_finite()).map(Some).ok_or_else(|| bad(cmd, format!("{key} is a number"))),
    }
}

fn add_placement(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.addPlacement";
    let id = one_piece(s, p, CMD)?;
    let name = str_param(p, "name").ok_or_else(|| bad(CMD, "name: what the design is called"))?;
    let at = p.get("at").and_then(point_of).ok_or_else(|| bad(CMD, "at: [x, y], the design's centre in document points"))?;
    let mut d = Placement::new(name, size_param(p, "widthMm", CMD)?, size_param(p, "heightMm", CMD)?, at).map_err(|e| err(CMD, e))?;
    d.kind = match str_param(p, "kind") {
        None | Some("embroidery") => DesignKind::Embroidery,
        Some("screen") => DesignKind::Screen,
        Some(_) => return Err(bad(CMD, "kind is \"embroidery\" or \"screen\"")),
    };
    if let Some(f) = str_param(p, "file") {
        d.file = f.chars().take(pf_core::placement::MAX_FILE_CHARS).collect();
    }
    if let Some(v) = number(p, "scale", CMD)? {
        d.set_scale(v).map_err(|e| err(CMD, e))?;
    }
    if let Some(v) = number(p, "rotation", CMD)? {
        d.set_rotation(v).map_err(|e| err(CMD, e))?;
    }
    if let Some(h) = p.get("hoop").filter(|h| !h.is_null()) {
        d.hoop = Some(hoop_param(h, CMD)?);
    }
    change(s, id, "Place Design", CMD, |piece| piece.add_placement(d).map_err(|e| err(CMD, e)))
}

fn index_of(p: &Value, piece: &Piece, cmd: &str) -> Result<usize> {
    let i = p.get("index").and_then(Value::as_u64).map(|i| i as usize);
    i.filter(|i| *i < piece.placements.len()).ok_or_else(|| bad(cmd, "index: the design's place in piece.info's placements"))
}

fn set_placement(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.setPlacement";
    let id = one_piece(s, p, CMD)?;
    change(s, id, "Move Placed Design", CMD, |piece| {
        let i = index_of(p, piece, CMD)?;
        let mut d = piece.placements[i].clone();
        if let Some(n) = str_param(p, "name") {
            d.name = pf_core::piece::check_name(n).map_err(|e| err(CMD, e))?;
        }
        if let Some(v) = p.get("at") {
            d.set_at(point_of(v).ok_or_else(|| bad(CMD, "at: [x, y] in document points"))?).map_err(|e| err(CMD, e))?;
        }
        if let Some(v) = number(p, "scale", CMD)? {
            d.set_scale(v).map_err(|e| err(CMD, e))?;
        }
        if let Some(v) = p.get("width") {
            d.set_true_width(mm_param(v).ok_or_else(|| bad(CMD, "width: mm"))?).map_err(|e| err(CMD, e))?;
        }
        if let Some(v) = p.get("height") {
            d.set_true_height(mm_param(v).ok_or_else(|| bad(CMD, "height: mm"))?).map_err(|e| err(CMD, e))?;
        }
        if let Some(v) = number(p, "rotation", CMD)? {
            d.set_rotation(v).map_err(|e| err(CMD, e))?;
        }
        match p.get("hoop") {
            None => {}
            Some(Value::Null) => d.hoop = None,
            Some(h) => d.hoop = Some(hoop_param(h, CMD)?),
        }
        piece.placements[i] = d;
        Ok(())
    })
}

fn remove_placement(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.removePlacement";
    let id = one_piece(s, p, CMD)?;
    change(s, id, "Remove Placed Design", CMD, |piece| {
        let i = index_of(p, piece, CMD)?;
        piece.placements.remove(i);
        Ok(())
    })
}
