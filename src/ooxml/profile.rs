//! Conformance vocabulary: ECMA-376 Part 1 §2 and Part 4 §2.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputProfile {
    #[default]
    Transitional,
    /// Reserved explicitly; compilation rejects it until implemented.
    Strict,
}
