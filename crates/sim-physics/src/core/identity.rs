use std::num::NonZeroU64;

use crate::Error;

macro_rules! identity {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
        pub struct $name(NonZeroU64);

        impl $name {
            /// Constructs an ID from the complete inclusive range `1..=u64::MAX`.
            pub fn new(value: u64) -> Result<Self, Error> {
                NonZeroU64::new(value).map(Self).ok_or(Error::ZeroId)
            }

            /// Returns the nonzero stable numeric identity.
            pub const fn get(self) -> u64 {
                self.0.get()
            }
        }
    };
}

identity!(
    BodyId,
    "Stable body identity, unrelated to storage or rendering order."
);
identity!(
    LinkId,
    "Stable relationship identity within one physical composition."
);
identity!(
    ForceSourceId,
    "Stable external force identity; never determines numeric summation order."
);
