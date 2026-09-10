# Foundation

This directory contains only domain-neutral contracts with multiple concrete
consumers. The current concrete set is typed units, the versioned constants
registry, stable identifiers, validated time values, exact bounded binary64
summation, and exact bounded product/quotient evaluation with one final
ties-to-even rounding.

Persistence API versioning does not exist yet. It belongs here only when a
real persisted schema needs it; it must not be introduced as a speculative
placeholder.

It must not depend on any domain, presentation library, Sim;Engine, GPU/window
library, UI framework, or external extension runtime. It is not a home for
generic helpers or types that merely look structurally similar.
