//! C9G-TYPES — the Phase 9 public types a consumer builds with: the
//! prelude's builder arguments, `text-align: justify-all` through the
//! builder, the typed `font-stretch` keywords and the `white-space`
//! important mask under its 0.5 name.

/// The prelude names the Phase 9 builders' arguments a typical app
/// writes: a stylesheet of aligned, spaced, decorated, bold text needs
/// nothing else imported.
mod prelude_only {
    use rdom_tui::prelude::*;

    #[test]
    fn the_prelude_has_the_text_builders_arguments() {
        let s = TuiStyle::new()
            .text_align(TextAlign::Center)
            .white_space(WhiteSpace::Pre)
            .line_height(LineHeight::Number(2.0))
            .vertical_align(VerticalAlign::Super)
            .text_decoration(TextDecoration::Underline)
            .text_decoration_line(TextDecorationLine::OVERLINE)
            .text_decoration_style(TextDecorationStyle::Wavy)
            .text_transform(TextTransform {
                case: TextCase::Uppercase,
                ..TextTransform::NONE
            })
            .font_weight(FontWeight::Bold)
            .font_style(FontStyle::Italic);
        assert_eq!(
            s.text.text_align_all,
            Some(Value::Specified(TextAlign::Center))
        );
        assert_eq!(s.font.weight, Some(Value::Specified(FontWeight::Bold)));
    }
}

use rdom_tui::*;

/// CSS Text 3 §6.1: `text-align: justify-all` is `text-align-all:
/// justify` with `text-align-last: justify` — the builder takes it as the
/// parser does, and reads back as written.
#[test]
fn the_text_align_builder_takes_justify_all() {
    let s = TuiStyle::new().text_align(TextAlignKeyword::JustifyAll);
    assert_eq!(
        s.text.text_align_all,
        Some(Value::Specified(TextAlign::Justify))
    );
    assert_eq!(
        s.text.text_align_last,
        Some(Value::Specified(TextAlignLast::Justify))
    );
    assert_eq!(
        TextAlignKeyword::JustifyAll.longhands(),
        (TextAlign::Justify, TextAlignLast::Justify)
    );
    assert_eq!(
        TextAlignKeyword::from(TextAlign::MatchParent).longhands(),
        (TextAlign::MatchParent, TextAlignLast::MatchParent)
    );
    let tokens = style::parse::tokenize("justify-all").unwrap();
    assert_eq!(
        style::parse::values::parse_text_align(&tokens),
        Some((TextAlign::Justify, TextAlignLast::Justify))
    );
}

/// CSS Fonts 4 §2.3: the width keywords are a closed set, each standing
/// for a percentage — a typed enum, so no string outside the grammar can
/// be stored.
#[test]
fn font_stretch_keywords_are_typed() {
    let tokens = style::parse::tokenize("condensed").unwrap();
    let parsed = style::parse::values::parse_font_stretch(&tokens);
    assert_eq!(
        parsed,
        Some(FontStretch::Keyword(FontStretchKeyword::Condensed))
    );
    assert_eq!(FontStretchKeyword::Condensed.percent(), 75.0);
    assert_eq!(
        FontStretchKeyword::UltraExpanded.keyword(),
        "ultra-expanded"
    );
    assert_eq!(FontStretchKeyword::ALL.len(), 8);
}

/// `ImportantMask::WHITE_SPACE`, the 0.5 name, is the shorthand's two
/// longhands, as `BOLD` / `ITALIC` are the font's.
#[test]
fn the_white_space_mask_keeps_its_name() {
    assert_eq!(
        ImportantMask::WHITE_SPACE,
        ImportantMask::WHITE_SPACE_COLLAPSE | ImportantMask::TEXT_WRAP_MODE
    );
}
