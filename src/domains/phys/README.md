# Sim;phys

This directory owns physical meaning: particles, bodies, forces, fields,
collisions, integrators, momentum, energy, and physical effect composition.

Subcategories share explicit physical contracts, not each other's private
state. Curated simulations select bounded effects; Phys;Sandbox may combine
compatible physical effects but must preserve numeric safety.

The product subdomains are Mechanics, Thermodynamics, Waves & Optics,
Electromagnetism, Relativity, Fluid Dynamics, and Sandbox. Mechanics is the
active vertical slice; the others stay separate until they have specifications.

Renderer particle, vector, field, and mesh types are forbidden here.
