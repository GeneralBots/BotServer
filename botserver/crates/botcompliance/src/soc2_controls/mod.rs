//! Default SOC 2 control catalog (Trust Service Criteria).
//! Data-only module split from soc2.rs for file-size limits.

use crate::soc2::Soc2Control;

mod part1;
mod part2;

/// The full default control catalog, both catalogue halves in order.
pub(crate) fn get_default_controls() -> Vec<Soc2Control> {
    let mut controls = part1::controls();
    controls.extend(part2::controls());
    controls
}
