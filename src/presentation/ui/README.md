# UI

This directory owns navigation, panels, editors, warnings, and interaction
state. It translates input into typed application intents and displays domain
read models and diagnostics.

It must not change domain stores, constants, or entities directly. Window and
event-loop integration stays on this side of the architecture boundary.

The current flow is `Sim;X -> Sim;Phys -> Phys;Mechanics -> editor`. Both menu
levels use smooth hover transitions. The host opens a dark borderless
full-screen window; this directory owns layout and interaction state but does
not import `sim_engine`.

The main menu also owns a deliberately hidden keyboard path: while the pointer
hovers the low-opacity corner `?`, numeric keys feed only the Sim;Time easter
egg sequence. Leaving the hotspot resets partial input and restores normal
domain shortcuts.
