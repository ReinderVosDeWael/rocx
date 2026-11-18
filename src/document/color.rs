//! Color and underline domain values, independent of XML spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeColor {
    Dark1,
    Light1,
    Dark2,
    Light2,
    Accent1,
    Accent2,
    Accent3,
    Accent4,
    Accent5,
    Accent6,
    Hyperlink,
    FollowedHyperlink,
    Background1,
    Text1,
    Background2,
    Text2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Rgb([u8; 3]),
    Auto,
    Theme {
        name: ThemeColor,
        tint: Option<u8>,
        shade: Option<u8>,
    },
}
impl From<[u8; 3]> for Color {
    fn from(value: [u8; 3]) -> Self {
        Self::Rgb(value)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Underline {
    None,
    Single,
    Double,
    Dotted,
    Dash,
    Wave,
    Words,
}
impl From<bool> for Underline {
    fn from(value: bool) -> Self {
        if value { Self::Single } else { Self::None }
    }
}
