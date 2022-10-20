use enum_iterator::Sequence;

#[derive(Debug, PartialEq, Sequence, Clone, Copy)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum CustomKeycode {
    DynamicTappingTermPrint,
    DynamicTappingTermIncrease,
    DynamicTappingTermDecrease,
}

impl From<CustomKeycode> for String {
    fn from(m: CustomKeycode) -> Self {
        match m {
            CustomKeycode::DynamicTappingTermPrint => "DT_PRNT",
            CustomKeycode::DynamicTappingTermIncrease => "DT_UP",
            CustomKeycode::DynamicTappingTermDecrease => "DT_DOWN",
        }
        .to_string()
    }
}
