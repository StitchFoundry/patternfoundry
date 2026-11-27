//! Pattern › Drape on Avatar in the background (PF-10.7): the drape (`drape.make`) is set up from
//! the document at once, falls on the background worker ([`crate::background`]) while editing goes
//! on, its progress shown with Cancel and Continue in Background (the export progress dialog,
//! counting its steps), and is drawn on the layer "Drape" of the document it was asked for when it
//! is done. Without a worker (the web) it runs at once, as `drape.make` does.

use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use vectorcraft_doc::cancel;
use vectorcraft_engine::cmd::drape::{Setup, finish, setup};
use vectorcraft_engine::cmd::fileio::progress::{self, Progress};

use crate::VectorcraftApp;

/// What the status bar and the progress dialog call it.
const LABEL: &str = "Draping on Avatar";

/// Start the drape of `p` (`drape.make`'s params, `background` aside) → `{background: true,
/// status}` while it runs, else the drape's result.
pub fn start(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    let mut params = p.clone();
    if let Some(o) = params.as_object_mut() {
        o.remove("background");
    }
    let ready = setup(&app.session, &params).map_err(|e| e.to_string())?;
    let uid = app.session.active().map(|d| d.uid).ok_or("no document")?;
    let steps = ready.steps as u64;
    let shared: Arc<Mutex<Option<Setup>>> = Arc::new(Mutex::new(Some(ready)));
    let (theirs, ours) = (shared.clone(), shared);
    let prog = Progress::new();
    let watched = prog.clone();
    let work = move |_: crate::background::Writer| -> Result<Value, String> {
        let mut drape = theirs.lock().map_err(|_| "the drape stopped")?.take().ok_or("the drape stopped")?;
        let done = progress::scope(&watched, || {
            drape.run(&mut |share| {
                cancel::progress((share * steps as f64).round() as u64, steps, "");
                !cancel::is_cancelled()
            })
        });
        if !done {
            return Ok(json!({ "cancelled": true }));
        }
        *theirs.lock().map_err(|_| "the drape stopped")? = Some(drape);
        Ok(Value::Null)
    };
    let then = move |app: &mut VectorcraftApp, r: Result<Value, String>| -> Result<Value, String> {
        if r?.get("cancelled").is_some() {
            app.status(crate::i18n::msg("The drape was cancelled"));
            return Ok(json!({ "cancelled": true }));
        }
        let drape = ours.lock().map_err(|_| "the drape stopped")?.take().ok_or("the drape stopped")?;
        // Drawn in the document it was asked for, brought to the front.
        let i = app.session.documents().iter().position(|d| d.uid == uid).ok_or("the document was closed")?;
        app.session.set_active(i);
        let out = finish(&mut app.session, &params, drape).map_err(|e| e.to_string())?;
        app.sync_views();
        // The drape is drawn beside the pattern, off screen: show it.
        crate::canvas::fit(app, "view.fitAll");
        app.status(crate::i18n::msg("Draped on the avatar"));
        Ok(out)
    };
    crate::background::run_with_steps(app, true, LABEL.into(), Some(uid), Some(prog), "Step {n} of {total}", work, then)
}
