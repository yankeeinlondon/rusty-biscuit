use strum::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// Form-control (radio / checkbox / check) icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Control {
    RadioUnselected,
    RadioSelected,
    RadioDisabled,
    RadioDisabledSelected,
    CircularCheck,
    CircularCheckUnread,
    CircularCheckOutline,
    CircularCheckOutlineUnread,
    SquareUnchecked,
    SquareChecked,
    SquareCheckedFill,
}

impl DomainIcon for Control {
    fn iconify_id(self) -> &'static str {
        match self {
            Control::RadioUnselected => "fluent:radio-button-24-regular",
            Control::RadioSelected => "fluent:radio-button-24-filled",
            Control::RadioDisabled => "fluent:radio-button-off-16-regular",
            Control::RadioDisabledSelected => "fluent:radio-button-off-16-filled",
            Control::CircularCheck => "material-symbols:check-circle-rounded",
            Control::CircularCheckUnread => "material-symbols:check-circle-unread",
            Control::CircularCheckOutline => "material-symbols:check-circle-outline-rounded",
            Control::CircularCheckOutlineUnread => {
                "material-symbols:check-circle-unread-outline-rounded"
            }
            Control::SquareUnchecked => "material-symbols:check-box-outline-blank",
            Control::SquareChecked => "material-symbols:check-box-outline-rounded",
            Control::SquareCheckedFill => "material-symbols:check-box-rounded",
        }
    }

    fn body(self) -> IconBody {
        body_for(self.iconify_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    #[test]
    fn display_string_round_trips_every_variant() {
        for variant in Control::iter() {
            let s = variant.to_string();
            assert_eq!(
                Control::from_str(&s).unwrap(),
                variant,
                "round trip for {s}"
            );
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Control::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
