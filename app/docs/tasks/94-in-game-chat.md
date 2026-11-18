# Task 94 — In-game chat that works

Crates: `rom-app` (input, chat state), `rom-render` (the chat panel). Implementer: Claude
(Sonnet).

## Why — the owner (2026-11-18)

"In-game chat (right now the control is inert)."

The HUD shows a chat line ("[Allies] Message…") under the event line; only the ascended
players' celestial channel (task 76) takes text, and typing by hand was never verified.

## Build

1. **Typing.** Click the chat line or press Enter: it takes the keyboard (game hotkeys pause
   while it has focus, the camera keys too); type; Enter sends; Escape cancels; Backspace,
   arrows, Home/End, Ctrl+V paste work; a caret shows. Shift+Enter or Tab switches the channel.
2. **Channels**: All, Allies (your team), and Celestial for ascended players (as now). The
   channel shows as a coloured tag; your choice is remembered for the match.
3. **The log**: sent and received lines appear with the sender's name in their player colour
   and the channel tag, newest at the bottom, fading after a while; a key (or a click on the
   chat icon in the top bar) opens the full history with scrolling. Lines are kept in the save
   (presentation only, outside the simulation and its hash), or say why not.
4. **Recipients.** Lines are presentation, never orders (rule 3). In a single-player match they
   go to the log; computer players do not answer (a later task may give allies replies). The
   chat goes through one `ChatSink` so network play (task 88) relays it without touching the UI.
5. Keep the ascension's celestial chat as it is, now typed through the same input.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/hud.md`, `app/docs/ascension.md` (chat),
`rom-app/src/sim_thread.rs` (chat draft and send), `ascension_view.rs`, `selection.rs` (keys),
the HUD's chat drawing in `rom-render`.

## Done when

- Gate green from `app/`; playthrough hash unchanged.
- Tests: focus takes the keys away from hotkeys and gives them back; Enter sends to the chosen
  channel; Escape keeps nothing; the Allies channel reaches only the team (by `ChatSink`);
  history keeps order.
- Screenshots you looked at (`app/docs/img/`, scripts `app/docs/tasks/94-*-script.txt`, typing
  driven by the script's `key` events): typing with a caret, the log with three lines from two
  channels, the history open.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes. Never use `xdotool`,
`wmctrl` or `sudo`; do not stop processes you did not start.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, deviations, anything not verified (typing with a real keyboard is the
owner's check).
