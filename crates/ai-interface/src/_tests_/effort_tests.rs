//! Supported efforts retain a nonempty, ordered set.

use crate::{Effort, EffortSet};

#[test]
fn supported_efforts_iterate_once_in_increasing_order() {
    let efforts = EffortSet::new(Effort::Max)
        .with(Effort::Disabled)
        .with(Effort::High)
        .with(Effort::High);

    assert_eq!(
        efforts.iter().collect::<Vec<_>>(),
        vec![Effort::Disabled, Effort::High, Effort::Max]
    );
}
