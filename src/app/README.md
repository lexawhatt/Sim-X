# Application Composition

This directory is the Sim;X composition root.

It may connect foundation contracts, domain APIs, and presentation adapters.
It owns lifecycle, simulation selection, command routing, pause/reset behavior,
and diagnostic coordination.

It must not contain domain formulas, renderer implementation, widget drawing,
or a speculative external extension runtime. Cross-domain scenarios are
coordinated here through explicit public ports rather than direct
domain-to-domain imports.
