//! Wire spelling for domain values: ECMA-376 Part 1 §§17–22.
use crate::document::*;
impl Alignment {
    pub(crate) fn xml(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
            Self::Justify => "both",
            Self::Distribute => "distribute",
            Self::JustifyMedium => "mediumKashida",
            Self::JustifyHigh => "highKashida",
            Self::JustifyLow => "lowKashida",
            Self::ThaiJustify => "thaiDistribute",
        }
    }
}
impl BreakType {
    pub(crate) fn xml(self) -> &'static str {
        match self {
            Self::Line => "textWrapping",
            Self::Page => "page",
            Self::Column => "column",
            Self::TextWrapping => "textWrapping",
        }
    }
}
impl Orientation {
    pub(crate) fn xml(self) -> &'static str {
        match self {
            Self::Portrait => "portrait",
            Self::Landscape => "landscape",
        }
    }
}
impl HeaderFooterType {
    pub(crate) fn xml(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::First => "first",
            Self::Even => "even",
        }
    }
}
impl VerticalAlignment {
    pub(crate) fn xml(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Center => "center",
            Self::Bottom => "bottom",
            Self::Both => "both",
        }
    }
}
impl SectionType {
    pub(crate) fn xml(self) -> &'static str {
        match self {
            Self::NextPage => "nextPage",
            Self::NextColumn => "nextColumn",
            Self::Continuous => "continuous",
            Self::EvenPage => "evenPage",
            Self::OddPage => "oddPage",
        }
    }
}
impl VerticalMerge {
    pub(crate) fn xml(self) -> &'static str {
        match self {
            Self::Restart => "restart",
            Self::Continue => "continue",
        }
    }
}
impl TableLayout {
    pub(crate) fn xml(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Autofit => "autofit",
        }
    }
}
impl BorderSide {
    pub(crate) fn xml(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Left => "left",
            Self::Bottom => "bottom",
            Self::Right => "right",
            Self::InsideH => "insideH",
            Self::InsideV => "insideV",
        }
    }
}
impl CellBorderSide {
    pub(crate) fn xml(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Start => "start",
            Self::End => "end",
        }
    }
}
impl BorderLine {
    pub(crate) fn xml(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Dashed => "dashed",
        }
    }
}
impl ThemeColor {
    pub(crate) fn xml(self) -> &'static str {
        match self {
            Self::Dark1 => "dark1",
            Self::Light1 => "light1",
            Self::Dark2 => "dark2",
            Self::Light2 => "light2",
            Self::Accent1 => "accent1",
            Self::Accent2 => "accent2",
            Self::Accent3 => "accent3",
            Self::Accent4 => "accent4",
            Self::Accent5 => "accent5",
            Self::Accent6 => "accent6",
            Self::Hyperlink => "hyperlink",
            Self::FollowedHyperlink => "followedHyperlink",
            Self::Background1 => "background1",
            Self::Text1 => "text1",
            Self::Background2 => "background2",
            Self::Text2 => "text2",
        }
    }
}
impl Underline {
    pub(crate) fn xml(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Single => "single",
            Self::Double => "double",
            Self::Dotted => "dotted",
            Self::Dash => "dash",
            Self::Wave => "wave",
            Self::Words => "words",
        }
    }
}
