# Ascension Privacy Policy

Ascension — Rise of Man is a game published by K3D0. This policy covers the game as
distributed in the Microsoft Store and on GitHub.

## What the game collects

Nothing. The game has no accounts, no analytics, no advertising and no telemetry. It does
not send anything about you or your computer to the publisher or to anyone else.

## What stays on your computer

- Your settings and launcher presets, in your user configuration folder.
- Saved games, in your user data folder.
- When you join a network game, the seat token for that server address, so that you can
  rejoin the same seat.

You can delete these files at any time; uninstalling the game does not send them anywhere.

## Network play

The game connects to the network only when you choose to: when you join a network match as a
client, when you host a match on your local network, or when you connect to a remote game host.
Nothing runs or listens on the network unless you start it.

- **Who you talk to.** A client talks only to the host address you type in. A host accepts only
  players who bring an access key.
- **Access keys.** Keys are issued by the game host and handed out by the match's organiser.
  Each key admits one player once and is then used up; the host keeps only a one-way
  fingerprint (SHA-256) of each key, never the key itself.
- **No accounts, anonymous names.** There are no accounts. Each player chooses the name shown
  in the match, and it can be any name.
- **What is sent.** Only the game itself: the orders you give, the name you chose, chat
  messages and the state of the match.
- **Encryption.** When the host's address starts with `https://` (as the publisher's own game
  host does), the connection is encrypted: HTTPS and secure WebSocket (`wss://`). A match on
  your own local network may use a plain, unencrypted connection (`ws://`) that stays inside
  that network.
- **Who can see it.** The person who runs a host can see what reaches it. When that host is the
  publisher's own server, the web server keeps standard access logs (IP address, time and page
  requested) to run and protect the server; they are not shared or sold and are used for
  nothing else.

No third party collects or stores data about you or your play: no advertising, analytics or
tracking service is involved, in the game or on the game's website (which sets no cookies).

## Children

The game collects no personal information from anyone, children included.

## Changes and contact

Changes to this policy are published with the game's source at
https://github.com/alfu32/ascension. Questions can be asked through the project's issue
tracker at https://github.com/alfu32/ascension/issues.
