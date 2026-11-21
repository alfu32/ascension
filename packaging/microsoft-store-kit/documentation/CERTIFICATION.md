# Certification notes for Ascension

Notes for the Microsoft Store certification testers.

- **What it is**: a real-time strategy game. Starting the app opens the launcher; choosing
  a setup such as **Quick match** and pressing **Play** starts a match against a computer player. No account,
  sign-in or network connection is needed to play.
- **Network**: optional. "Host a game", "Join a game" and "Run a server" use direct
  connections (WebSocket over HTTP or HTTPS) to an address the player types; nothing is
  contacted otherwise.
- **Package**: a full-trust desktop application (`runFullTrust`) with the executables
  `rise-of-man.exe` (the game) and `rom.exe` (a command-line runner and server, not a start
  entry). Everything the game reads at run time is built into the executables.
- **Graphics**: DirectX 11 (window and screens) and OpenGL 3.3 (the 3D world, drawn
  offscreen). A machine without OpenGL 3.3 drivers cannot draw the world.
- **Input**: keyboard and mouse. The interface is in English.
- **Saves and settings**: written to the user's own folders (`%APPDATA%`).
- **Content**: historical and futuristic warfare between small figures; no blood, no
  real-world people, no user-generated content shared through the publisher.
