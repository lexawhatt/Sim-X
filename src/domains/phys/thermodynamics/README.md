# Sim;phys;Thermodynamics

This subdomain owns temperature, thermal energy, heat capacity, conductive
relationships, and thermodynamic read models. Its first slice is the bounded
lumped-capacitance conduction model specified in
`docs/domains/phys/THERMODYNAMICS_CONDUCTION.md`.

It may use domain-neutral foundation contracts. It must not depend directly on
Mechanics, another Physics subdomain, presentation, or a concrete renderer.
Ideal gases, diffusion, phase transitions, and Brownian motion remain separate
future contracts rather than hidden additions to the conduction model.
