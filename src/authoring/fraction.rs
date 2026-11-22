use crate::document::setters;
use crate::{Condition, Inline, Value};

/// A stacked fraction in a body paragraph, with independently deferred text operands.
#[derive(Debug)]
pub struct InlineFraction {
    pub numerator: Value<String>,
    pub denominator: Value<String>,
    pub condition: Condition,
}

setters!(InlineFraction {
    with_numerator => numerator: into<Value<String>>,
    with_denominator => denominator: into<Value<String>>,
});

impl InlineFraction {
    pub fn new(numerator: impl Into<Value<String>>, denominator: impl Into<Value<String>>) -> Self {
        Self {
            numerator: numerator.into(),
            denominator: denominator.into(),
            condition: Condition::default(),
        }
    }

    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }
}

impl From<InlineFraction> for Inline {
    fn from(value: InlineFraction) -> Self {
        Self::InlineFraction(value)
    }
}

impl From<InlineFraction> for Value<Inline> {
    fn from(value: InlineFraction) -> Self {
        Self::ready(value.into())
    }
}
