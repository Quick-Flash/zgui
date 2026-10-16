//! The break a context was kept at, and the step that puts each shaped form back on it.
//!
//! A shaped paragraph holds one break at a time, and paint reads its glyphs from that break. The
//! kept pass leaves the form on the lines it resolved to. A size probe that is not answered from a
//! remembered break moves the form to the probe's width, and a later kept layout served from the
//! engine's cache runs no pass that moves it back. So every probe notes its box, and the end of the
//! layout pass breaks each noted box's form at its kept request again where the form moved.

use std::sync::Arc;

use zgui_dom::side::BoxKey;
use zgui_geom::CssPx;
use zgui_profile::{Counter, counter};
use zgui_text::{BreakRequest, InlineBoxGeometry, LineBand, LineBands};
use zgui_text_style::BreakingKey;

use crate::inline::content::Generated;
use crate::inline::resolved::InlineResolution;
use crate::measure::MeasureContent;
use crate::tree::store::LayoutStore;

/// The request a context's lines were broken under.
///
/// Holds no allocation of its own for a context with no atomic inline and no float beside it,
/// which is nearly every context.
#[derive(Clone, Debug)]
pub struct KeptBreak {
    /// The flattened content: the runs and the paragraph style.
    pub(crate) generated: Arc<Generated>,
    /// The atomic inlines, at the geometry of the last break.
    pub(crate) boxes: Vec<InlineBoxGeometry>,
    /// The width the lines were broken into.
    pub(crate) max_advance: Option<CssPx>,
    /// The width a percentage indent was resolved against.
    pub(crate) indent_basis: Option<CssPx>,
    /// The width of each line beside a float.
    pub(crate) bands: Vec<LineBand>,
    /// The key the form reflected right after the kept break, which saves hashing the request
    /// again. `None` for lines a probe left, whose break may have been recalled.
    pub(crate) key: Option<BreakingKey>,
}

impl KeptBreak {
    /// The request, as the kept pass made it.
    pub(crate) fn request(&self) -> BreakRequest<'_> {
        BreakRequest {
            runs: &self.generated.runs,
            boxes: &self.boxes,
            paragraph: &self.generated.paragraph,
            max_advance: self.max_advance,
            indent_basis: self.indent_basis,
            bands: LineBands::new(&self.bands),
            probe: false,
        }
    }

    /// The key of the request, as the kept pass made it.
    pub(crate) fn key(&self) -> BreakingKey {
        self.key.unwrap_or_else(|| self.request().key())
    }
}

/// Breaks each probed context's form at its kept request again, where a probe moved it.
///
/// Called once at the end of a layout pass. A box that was probed and whose form still reflects
/// its kept break costs one key computation and one lookup.
pub(crate) fn restore<C: MeasureContent>(store: &mut LayoutStore, content: &mut C) {
    let probed = core::mem::take(&mut store.probed);
    for key in probed {
        let Some(state) = store.state_opt_mut(key) else {
            continue;
        };
        state.probed = false;
        let Some(resolution) = state.inline.as_deref() else {
            continue;
        };
        if restore_one(resolution, content) {
            tracing::debug!(target: "zgui::layout", box_ = key.index(), "text form restored");
        }
    }
}

/// Breaks one context's form at its kept request, if the form reflects another break.
///
/// Returns whether it did.
fn restore_one<C: MeasureContent>(resolution: &InlineResolution, content: &mut C) -> bool {
    match content.breaks_reflect(resolution.key) {
        Some(reflected) if reflected != resolution.kept.key() => {
            content.break_lines(resolution.key, &resolution.kept.request());
            counter::bump(Counter::TextFormsRestored);
            true
        }
        _ => false,
    }
}

/// Every context with a form of its own that reflects a break other than its kept one.
///
/// Empty after a layout pass. The check runs under `ZGUI_INVARIANTS`, at the end of every pass.
pub(crate) fn mismatched<C: MeasureContent>(store: &LayoutStore, content: &C) -> Vec<BoxKey> {
    let mut out = Vec::new();
    let Some(root) = store.root() else {
        return out;
    };
    let mut stack = vec![root];
    while let Some(key) = stack.pop() {
        if let Some(resolution) = store.inline_resolution(key)
            && store.paragraph_users(resolution.paragraph) == 1
            && content
                .breaks_reflect(resolution.key)
                .is_some_and(|reflected| reflected != resolution.kept.key())
        {
            out.push(key);
        }
        stack.extend(store.node(key).children.iter().copied());
    }
    out
}

/// A description of each box and the chain of boxes above it, for the invariant's message.
pub(crate) fn describe(store: &LayoutStore, keys: &[BoxKey]) -> String {
    let mut out = String::new();
    for &key in keys {
        out.push_str(&format!("\n  box {}:", key.index()));
        let mut at = Some(key);
        while let Some(current) = at {
            let node = store.node(current);
            out.push_str(&format!(" <- {}:{:?}", current.index(), node.fc));
            at = node.parent;
        }
    }
    out
}
