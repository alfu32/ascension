// Ascension public site: one piece of music at a time, and the sound buttons of the sounds page.
"use strict";
document.documentElement.classList.add("js");

// Starting one player pauses the others (the story page's music).
document.addEventListener("play", (event) => {
  for (const audio of document.querySelectorAll("audio")) {
    if (audio !== event.target) audio.pause();
  }
  stopCurrent();
}, true);

// Sounds are bundled per unit or building type into one Ogg file ("sprite"); the index gives
// every sound key its sprite, start and length in seconds, and the page plays that stretch with
// Web Audio (an audio element seeked to the stretch as a fallback).
const dataNode = document.getElementById("sound-data");
const DATA = dataNode ? JSON.parse(dataNode.textContent) : { index: {}, urls: {} };
const fallback = new Audio();
fallback.preload = "none";
let audioCtx = null;
let current = null;
let playingBtn = null;
let stopTimer = null;
const buffers = {};

function stopCurrent() {
  if (current) { try { current.stop(); } catch (_) { /* already ended */ } current = null; }
  if (!fallback.paused) fallback.pause();
  clearTimeout(stopTimer);
  if (playingBtn) playingBtn.setAttribute("aria-pressed", "false");
  playingBtn = null;
}

function sprite(name) {
  if (!buffers[name]) {
    buffers[name] = fetch(DATA.urls[name])
      .then((r) => { if (!r.ok) throw new Error(String(r.status)); return r.arrayBuffer(); })
      .then((buf) => audioCtx.decodeAudioData(buf))
      .catch((e) => { delete buffers[name]; throw e; });
  }
  return buffers[name];
}

async function playKey(key, btn) {
  if (playingBtn === btn) { stopCurrent(); return; }
  for (const audio of document.querySelectorAll("audio")) audio.pause();
  stopCurrent();
  const at = DATA.index[key];
  if (!at) return;
  btn.setAttribute("aria-pressed", "true");
  playingBtn = btn;
  try {
    audioCtx = audioCtx || new (window.AudioContext || window.webkitAudioContext)();
    if (audioCtx.state === "suspended") await audioCtx.resume();
    const buffer = await sprite(at[0]);
    if (playingBtn !== btn) return;
    const src = audioCtx.createBufferSource();
    src.buffer = buffer;
    src.connect(audioCtx.destination);
    src.onended = () => {
      if (current === src) { current = null; if (playingBtn === btn) { btn.setAttribute("aria-pressed", "false"); playingBtn = null; } }
    };
    current = src;
    src.start(0, at[1], at[2]);
  } catch (_) {
    if (playingBtn !== btn) return;
    fallback.src = DATA.urls[at[0]];
    fallback.addEventListener("loadedmetadata", () => {
      fallback.currentTime = at[1];
      fallback.play().then(() => {
        stopTimer = setTimeout(() => { if (playingBtn === btn) stopCurrent(); }, at[2] * 1000);
      }, () => { if (playingBtn === btn) stopCurrent(); });
    }, { once: true });
    fallback.load();
  }
}

for (const btn of document.querySelectorAll("button.play[data-keys]")) {
  const keys = btn.dataset.keys.split(" ");
  btn.addEventListener("click", () => playKey(keys[Math.floor(Math.random() * keys.length)], btn));
}
