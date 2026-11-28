//! The "Share it on MakerFoundry?" prompt shown after a project's first successful export or
//! print (#165): when to offer it, and what each answer changes. State only: no network code, no
//! drawing. The app shows the prompt, asks the Foundry Account to sign in only after Share or Sell,
//! and opens the MakerFoundry draft page; publishing, price and Seller Terms happen there.

use serde_json::{Value, json};

/// What the project remembers about the prompt. Saved with the project, so it is asked once per
/// project, however many times the project is opened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShareState {
    /// The prompt has been shown for this project.
    pub shown: bool,
}

/// How the user answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    ShareFree,
    Sell,
    NotNow,
    /// The prompt was closed (Esc, the close button) with no answer: the same as Not now.
    Dismissed,
}

/// What the app does after an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Sign in if needed and open the draft page; `Some(true)` is the "Sell it" draft.
    pub open_draft: Option<bool>,
    /// "Offer to share after export" goes off (the "Don't ask again" box was ticked).
    pub turn_setting_off: bool,
}

impl ShareState {
    /// Whether to show the prompt after an export or print that just finished. `exported` is false
    /// when it failed or was cancelled, so a failed export never uses up the one offer. The
    /// export itself never waits for this.
    pub fn should_offer(&self, setting_on: bool, exported: bool) -> bool {
        setting_on && exported && !self.shown
    }

    /// Note that the prompt is on screen, so a second export in the same session, or a reopened
    /// project, does not ask again.
    pub fn mark_shown(&mut self) {
        self.shown = true;
    }

    /// What an answer does. Share and Sell need the "I made this / I own the rights" box ticked:
    /// without it nothing opens (`owns` false), and the prompt stays used up.
    pub fn answer(&mut self, choice: Choice, owns: bool, dont_ask_again: bool) -> Outcome {
        self.shown = true;
        let open_draft = match choice {
            Choice::ShareFree if owns => Some(false),
            Choice::Sell if owns => Some(true),
            _ => None,
        };
        Outcome { open_draft, turn_setting_off: dont_ask_again }
    }

    pub fn to_json(&self) -> Value {
        json!({"shown": self.shown})
    }

    /// Reads what [`ShareState::to_json`] wrote; anything else (an old project, a damaged file)
    /// reads as not shown yet.
    pub fn from_json(v: &Value) -> ShareState {
        ShareState { shown: v.get("shown").and_then(Value::as_bool).unwrap_or(false) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offered_once_per_project() {
        let mut s = ShareState::default();
        assert!(s.should_offer(true, true));
        s.mark_shown();
        assert!(!s.should_offer(true, true), "a second export does not ask again");
        let reopened = ShareState::from_json(&s.to_json());
        assert!(!reopened.should_offer(true, true), "nor does a reopened project");
    }

    #[test]
    fn a_new_project_is_asked_again() {
        let mut a = ShareState::default();
        a.mark_shown();
        assert!(ShareState::default().should_offer(true, true));
    }

    #[test]
    fn setting_off_never_offers() {
        assert!(!ShareState::default().should_offer(false, true));
    }

    #[test]
    fn a_failed_or_cancelled_export_keeps_the_offer() {
        let s = ShareState::default();
        assert!(!s.should_offer(true, false));
        assert!(s.should_offer(true, true));
    }

    #[test]
    fn dont_ask_again_turns_the_setting_off() {
        for choice in [Choice::ShareFree, Choice::Sell, Choice::NotNow, Choice::Dismissed] {
            let mut s = ShareState::default();
            let o = s.answer(choice, true, true);
            assert!(o.turn_setting_off, "{choice:?}");
            assert!(s.shown);
            let mut s = ShareState::default();
            assert!(!s.answer(choice, true, false).turn_setting_off);
        }
    }

    #[test]
    fn share_and_sell_open_the_matching_draft_only_when_owned() {
        let mut s = ShareState::default();
        assert_eq!(s.answer(Choice::ShareFree, true, false).open_draft, Some(false));
        assert_eq!(s.answer(Choice::Sell, true, false).open_draft, Some(true));
        assert_eq!(s.answer(Choice::ShareFree, false, false).open_draft, None);
        assert_eq!(s.answer(Choice::Sell, false, false).open_draft, None);
    }

    #[test]
    fn not_now_and_closing_open_nothing_and_use_up_the_offer() {
        for choice in [Choice::NotNow, Choice::Dismissed] {
            let mut s = ShareState::default();
            assert_eq!(s.answer(choice, true, false).open_draft, None);
            assert!(!s.should_offer(true, true));
        }
    }

    #[test]
    fn damaged_state_reads_as_not_shown() {
        for v in [json!(null), json!({}), json!({"shown": "yes"}), json!([1])] {
            assert_eq!(ShareState::from_json(&v), ShareState::default());
        }
    }
}
