//! The effects of nested inline boxes, painted with the line they are on.
//!
//! A nested inline box has no fragment of its own size. The line that holds its text paints its
//! background, border and shadows under the glyphs, and its decorations with them, from the parts
//! layout recorded on the line. A box broken across lines is sliced: a part the box continues past
//! has no border and no rounded corners on that side (`box-decoration-break: slice`).

use zgui_css::ComputedStyle;
use zgui_geom::{Corners, Device, DevicePx, Edges, Rect, Vec2};
use zgui_layout::inline::slices::{InlineSlice, device_rect};
use zgui_scene::{ClipId, Scene, SpatialId};

use crate::emit::box_::{self, BoxPlacement};
use crate::lower::PaintStyle;

/// One part of a nested inline box, ready to paint.
#[derive(Clone, Debug)]
pub struct InlinePaint {
    /// The box's lowered style, with its running animations composed in.
    pub style: PaintStyle,
    /// Where the part's background and border go.
    pub placement: BoxPlacement,
    /// Where its text sits, for its decorations and its glyphs.
    pub text: InlineText,
}

/// Where the text of one part of a nested inline box sits, in absolute device pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InlineText {
    /// The left edge of the part's content.
    pub left: f32,
    /// The right edge of the part's content.
    pub right: f32,
    /// The box's baseline, after its shift.
    pub baseline: f32,
    /// The box's font ascent.
    pub ascent: f32,
    /// The box's font descent.
    pub descent: f32,
    /// The box's font x-height.
    pub x_height: f32,
    /// The box's font size.
    pub font_size: f32,
    /// How far the box raises its text above the line's baseline.
    pub shift: f32,
    /// Where the box's range of the generated string starts.
    pub start: u32,
    /// Where it ends.
    pub end: u32,
}

impl InlinePaint {
    /// One part, placed against the content box of the context that holds it.
    pub fn of(
        slice: &InlineSlice,
        style: PaintStyle,
        computed: &ComputedStyle,
        content_box: Rect<DevicePx, Device>,
        clip: ClipId,
        transform: SpatialId,
        scale: f32,
    ) -> Self {
        let border_box = device_rect(content_box, slice);
        let radii = sliced(
            crate::lower::border::radii_of(computed, border_box, scale),
            slice.first,
            slice.last,
        );
        let (left, right) = slice.content_x();
        Self {
            style,
            placement: BoxPlacement {
                border_box,
                border: Edges {
                    top: DevicePx(slice.border.top),
                    right: DevicePx(slice.border.right),
                    bottom: DevicePx(slice.border.bottom),
                    left: DevicePx(slice.border.left),
                },
                radii,
                clip,
                transform,
                scale,
            },
            text: InlineText {
                left: content_box.origin.x.0 + left,
                right: content_box.origin.x.0 + right,
                baseline: content_box.origin.y.0 + slice.baseline,
                ascent: slice.ascent,
                descent: slice.descent,
                x_height: slice.x_height,
                font_size: slice.font_size,
                shift: slice.shift,
                start: slice.text.start,
                end: slice.text.end,
            },
        }
    }
}

/// The corner radii of a part: square on each side the box continues past.
fn sliced(mut radii: Corners<Vec2<DevicePx>>, first: bool, last: bool) -> Corners<Vec2<DevicePx>> {
    if !first {
        radii.top_left = Vec2::default();
        radii.bottom_left = Vec2::default();
    }
    if !last {
        radii.top_right = Vec2::default();
        radii.bottom_right = Vec2::default();
    }
    radii
}

/// Emits the shadows, backgrounds and borders of every part, in tree order, and returns how many
/// primitives were pushed.
///
/// `alpha` is the folded group alpha every colour is multiplied by.
pub fn boxes(
    scene: &mut Scene,
    parts: &[InlinePaint],
    alpha: f32,
    shaders: &dyn crate::content::shader::ShaderSource,
    pointer: Option<zgui_geom::Point<DevicePx, Device>>,
) -> usize {
    let mut pushed = 0;
    for part in parts {
        if !part.style.visible || part.style.paints_nothing() {
            continue;
        }
        let faded = crate::walk::order::faded(&part.style, alpha);
        pushed += box_::outer_shadows(scene, &faded, part.placement);
        pushed += box_::background_and_border(scene, &faded, part.placement, shaders, pointer);
        pushed += box_::inset_shadows(scene, &faded, part.placement);
    }
    pushed
}

#[cfg(test)]
mod tests {
    use zgui_css::StyleDraft;
    use zgui_geom::{Corners, DevicePx, Point, Rect, Size, Vec2};
    use zgui_layout::inline::slices::InlineSlice;
    use zgui_scene::{ClipId, SpatialId};

    use super::{InlinePaint, sliced};

    /// A part twenty pixels in and four down, forty wide and twenty tall, with a 2 px border and a
    /// 3 px padding on each side it holds.
    fn slice(first: bool, last: bool) -> InlineSlice {
        let side = |held: bool, width: f32| if held { width } else { 0.0 };
        InlineSlice {
            box_: zgui_layout::BoxKey::new(
                1,
                zgui_arena::Generation::FIRST,
                zgui_arena::DomainId::FIRST,
            ),
            line: 0,
            x: 20.0,
            y: 4.0,
            width: 40.0,
            height: 20.0,
            border: zgui_geom::Edges {
                left: side(first, 2.0),
                right: side(last, 2.0),
                top: 2.0,
                bottom: 2.0,
            },
            padding: zgui_geom::Edges {
                left: side(first, 3.0),
                right: side(last, 3.0),
                top: 0.0,
                bottom: 0.0,
            },
            first,
            last,
            text: 3..7,
            shift: 0.0,
            baseline: 18.0,
            ascent: 12.0,
            descent: 4.0,
            x_height: 7.0,
            font_size: 16.0,
        }
    }

    /// Paints `slice` against a content box at (100, 50).
    fn paint(slice: &InlineSlice) -> InlinePaint {
        let computed = StyleDraft::initial().build();
        let style = crate::lower::lower(&computed, 1.0);
        InlinePaint::of(
            slice,
            style,
            &computed,
            Rect::new(
                Point::new(DevicePx(100.0), DevicePx(50.0)),
                Size::new(DevicePx(400.0), DevicePx(100.0)),
            ),
            ClipId::ROOT,
            SpatialId::VIEWPORT,
            1.0,
        )
    }

    #[test]
    fn a_part_is_placed_against_the_content_box() {
        let part = paint(&slice(true, true));
        assert_eq!(
            part.placement.border_box,
            Rect::new(
                Point::new(DevicePx(120.0), DevicePx(54.0)),
                Size::new(DevicePx(40.0), DevicePx(20.0)),
            )
        );
        assert_eq!(part.text.left, 125.0, "inside the border and the padding");
        assert_eq!(part.text.right, 155.0);
        assert_eq!(part.text.baseline, 68.0);
    }

    #[test]
    fn a_broken_side_has_no_border_and_no_rounded_corners() {
        let middle = paint(&slice(false, false));
        assert_eq!(middle.placement.border.left, DevicePx(0.0));
        assert_eq!(middle.placement.border.right, DevicePx(0.0));
        assert_eq!(middle.placement.border.top, DevicePx(2.0));
        let first = paint(&slice(true, false));
        assert_eq!(first.placement.border.left, DevicePx(2.0));
        assert_eq!(
            first.text.right, 160.0,
            "the content runs to the broken edge"
        );

        let round = Corners::new(
            Vec2::splat(DevicePx(6.0)),
            Vec2::splat(DevicePx(6.0)),
            Vec2::splat(DevicePx(6.0)),
            Vec2::splat(DevicePx(6.0)),
        );
        let start = sliced(round, true, false);
        assert_eq!(start.top_left, round.top_left);
        assert_eq!(start.bottom_left, round.bottom_left);
        assert_eq!(start.top_right, Vec2::default());
        assert_eq!(start.bottom_right, Vec2::default());
        let end = sliced(round, false, true);
        assert_eq!(end.top_left, Vec2::default());
        assert_eq!(end.top_right, round.top_right);
    }
}
