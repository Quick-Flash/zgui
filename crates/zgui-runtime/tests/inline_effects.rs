//! What a nested inline box paints with the line it is on.
//!
//! The fixed face advances eight pixels a character, so every rectangle below is arithmetic: the
//! chip after `aa ` starts 24 pixels in, its padding is 4 pixels either side and its four
//! characters are 32 pixels wide.

mod support;

use zgui_interned::ClassName;
use zgui_reactive::RwSignal;
use zgui_reactive::prelude::*;
use zgui_view::{BuildCx, IntoView, View};

const CSS: &str = ":root { display: block; width: 400px; height: 300px }
                   .para { display: block }
                   .chip { padding: 0 4px; background-color: rgb(255, 0, 0) }
                   .chip.on { background-color: rgb(0, 0, 255) }
                   .under { text-decoration: underline rgb(0, 255, 0) }";

/// A solid quad of the frame: its painting order, its colour as 8-bit sRGB and its rectangle.
fn quads(scene: &zgui_scene::Scene) -> Vec<(u32, [u8; 3], [f32; 4])> {
    scene
        .primitives
        .quads
        .iter()
        .filter_map(|quad| match scene.paints.get(quad.fill.id()?) {
            Some(zgui_scene::Paint::Solid(color)) => {
                let rgb = color
                    .to_space(zgui_color::ColorSpace::Srgb)
                    .components()
                    .map(|channel| (channel * 255.0).round() as u8);
                Some((quad.order.into(), rgb, quad.bounds))
            }
            _ => None,
        })
        .collect()
}

/// A paragraph of `aa ` and a chip holding `bbbb`, with `chip` deciding the chip's classes.
fn chip_view(
    on: RwSignal<bool>,
    chip_class: &'static str,
) -> impl FnMut(&mut BuildCx<'_>) -> Box<dyn zgui_view::Anchor> {
    move |cx: &mut BuildCx<'_>| {
        Box::new(
            zgui_elements::r#box()
                .class("para")
                .child(zgui_elements::text().child(|| "aa "))
                .child(
                    zgui_elements::text()
                        .class(chip_class)
                        .class_toggle(ClassName::new("on"), move || on.get())
                        .child(|| "bbbb"),
                )
                .into_view()
                .build(cx),
        ) as Box<dyn zgui_view::Anchor>
    }
}

#[test]
fn a_chip_is_painted_under_its_glyphs_across_its_own_range() {
    let on = RwSignal::new(false);
    let mut harness = support::app_with_text(CSS, chip_view(on, "chip"));
    harness.settle(8);
    let window = &harness.app().windows()[0];
    let line = support::first_line_box(window);
    let scene = window.scene();

    let chips: Vec<_> = quads(scene)
        .into_iter()
        .filter(|(_, rgb, _)| *rgb == [255, 0, 0])
        .collect();
    assert_eq!(chips.len(), 1, "one chip, one quad: {chips:?}");
    let (order, _, bounds) = chips[0];
    assert_eq!(
        bounds[0] - line.origin.x.0,
        24.0,
        "the chip starts after `aa `"
    );
    assert_eq!(
        bounds[2],
        4.0 + 32.0 + 4.0,
        "the padding, then four characters"
    );

    let glyphs: Vec<_> = scene
        .primitives
        .mono_sprites
        .iter()
        .map(|sprite| (u32::from(sprite.order), sprite.bounds))
        .collect();
    assert!(!glyphs.is_empty(), "the line drew no glyphs");
    // The order is a painting order over overlaps: a glyph clear of the chip may share its place.
    let over = |glyph: &[f32; 4]| {
        glyph[0] < bounds[0] + bounds[2]
            && bounds[0] < glyph[0] + glyph[2]
            && glyph[1] < bounds[1] + bounds[3]
            && bounds[1] < glyph[1] + glyph[3]
    };
    assert!(
        glyphs
            .iter()
            .filter(|(_, glyph)| over(glyph))
            .all(|(glyph, _)| *glyph > order),
        "the chip is painted over its glyphs: chip at {order}, glyphs {glyphs:?}"
    );
    assert!(
        glyphs
            .iter()
            .any(|(_, glyph)| glyph[0] - line.origin.x.0 == 24.0 + 4.0),
        "the chip's first glyph sits inside its padding: {glyphs:?}"
    );
    harness.shut_down();
}

#[test]
fn a_chip_that_changes_colour_is_painted_in_the_new_one() {
    let on = RwSignal::new(false);
    let mut harness = support::app_with_text(CSS, chip_view(on, "chip"));
    harness.settle(8);

    for step in 0..6 {
        on.set(step % 2 == 0);
        harness.settle(8);
        let want = if step % 2 == 0 {
            [0, 0, 255]
        } else {
            [255, 0, 0]
        };
        let colours: Vec<_> = quads(harness.app().windows()[0].scene())
            .into_iter()
            .map(|(_, rgb, _)| rgb)
            .filter(|rgb| *rgb == [255, 0, 0] || *rgb == [0, 0, 255])
            .collect();
        assert_eq!(
            colours,
            [want],
            "step {step}: the line replayed the chip's old colour"
        );
    }
    harness.shut_down();
}
