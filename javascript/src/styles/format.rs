//! Share eager text-layout DTOs without creating additional deferred values.
use crate::{boundary::Pass, host::Wire, text_layout};
use rocx::{Alignment, LineHeight, ParagraphFormat, Result, RunFormat, RunLanguage, TabStop};

pub(super) use text_layout::{
    alignment_view, line_height_view, paragraph_format_view, rgb_view, run_format_view,
    run_language_view, tab_stop_view,
};

macro_rules! eager {
    ($($name:ident: $ty:ty),+ $(,)?) => {$(
        pub(super) fn $name(input: &Wire, _: &Pass, path: &str) -> Result<$ty> {
            text_layout::$name(input, path)
        }
    )+};
}

eager! {
    alignment: Alignment,
    line_height: LineHeight,
    paragraph_format: ParagraphFormat,
    rgb: [u8; 3],
    run_format: RunFormat,
    run_language: RunLanguage,
    tab_stop: TabStop,
}
