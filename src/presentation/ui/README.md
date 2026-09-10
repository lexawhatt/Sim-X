# UI

This directory owns navigation, panels, editors, warnings, and interaction
state. It translates input into typed application intents and displays domain
read models and diagnostics.

It must not change domain stores, constants, or entities directly. Window and
event-loop integration stays on this side of the architecture boundary.

The current flow is
`Sim;X -> Sim;Phys -> Phys subdomain -> Projects -> Editor -> View`. Menu and
workspace controls use smooth hover transitions. The host opens a dark
borderless full-screen window; this directory owns layout and interaction state
but does not import `sim_engine`.

Editor and View are separate permission states. Editor exposes authoring tools
and the Inspector, but never playback targets or simulation stepping. View
exposes measurements and playback, but never structural editing or the
Inspector. Leaving View first pauses and opens an Apply/Discard/Cancel decision:
Apply keeps the final View state as the new Editor baseline, Discard restores
the exact pre-View session clone, and Cancel stays in View.

The current Mechanics editor emits typed body placement, selection, movement,
mass-scaling, and persistent-force commands. A Force click with no drag clears
the authored force. Thermal, Wave, and Electrostatic editor pages truthfully
show curated starter setups but do not expose structural edit affordances yet.

The main menu also owns a deliberately hidden keyboard path: while the pointer
hovers the low-opacity corner `?`, numeric keys feed only the Sim;Time easter
egg sequence. Leaving the hotspot resets partial input and restores normal
domain shortcuts.
