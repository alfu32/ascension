//! The audio thread: decodes the sounds, opens the output device and feeds the mixer's output to
//! it. Calls from the game only send commands; nothing here blocks the caller.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

use crate::decode::SoundBank;
use crate::mixer::{Command, Mixer, Volumes};
use crate::placement::Placement;
use crate::table::SoundId;

/// What the engine is doing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineStatus {
    /// Decoding the sounds and opening the device.
    Starting,
    /// Playing through `device` at `rate` frames per second.
    Running { device: String, rate: u32 },
    /// Accepting calls without playing: no device, a device error, or made silent on purpose.
    Silent { reason: String },
}

struct Shared {
    status: Mutex<EngineStatus>,
    running: AtomicBool,
    frames: AtomicU64,
}

impl Shared {
    fn set(&self, status: EngineStatus) {
        self.running.store(
            matches!(status, EngineStatus::Running { .. }),
            Ordering::Release,
        );
        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = status;
    }
}

/// Sound output. Create one with [`AudioEngine::new`]; it is silent, never failing, when the
/// machine has no audio device.
pub struct AudioEngine {
    commands: Option<Sender<Command>>,
    shared: Arc<Shared>,
    stop: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl AudioEngine {
    /// Start the audio thread: it decodes every sound, opens the default output device and
    /// plays. Returns at once; [`AudioEngine::status`] tells how it went. One-shots asked for
    /// before the device runs are dropped; loops and volumes are kept.
    pub fn new() -> AudioEngine {
        let shared = Arc::new(Shared {
            status: Mutex::new(EngineStatus::Starting),
            running: AtomicBool::new(false),
            frames: AtomicU64::new(0),
        });
        let (commands, receiver) = mpsc::channel();
        let (stop, stopped) = mpsc::channel();
        let thread_shared = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("audio".into())
            .spawn(move || run(receiver, stopped, thread_shared));
        match thread {
            Ok(handle) => AudioEngine {
                commands: Some(commands),
                shared,
                stop: Some(stop),
                thread: Some(handle),
            },
            Err(e) => AudioEngine::silent_because(format!("no audio thread: {e}")),
        }
    }

    /// An engine that plays nothing and accepts every call.
    pub fn silent() -> AudioEngine {
        AudioEngine::silent_because("made silent".into())
    }

    fn silent_because(reason: String) -> AudioEngine {
        AudioEngine {
            commands: None,
            shared: Arc::new(Shared {
                status: Mutex::new(EngineStatus::Silent { reason }),
                running: AtomicBool::new(false),
                frames: AtomicU64::new(0),
            }),
            stop: None,
            thread: None,
        }
    }

    /// What the engine is doing.
    pub fn status(&self) -> EngineStatus {
        self.shared
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Frames sent to the device so far.
    pub fn frames_played(&self) -> u64 {
        self.shared.frames.load(Ordering::Relaxed)
    }

    /// Volume settings, each 0…1.
    pub fn set_volumes(&self, master: f32, effects: f32, voice: f32, ambient: f32) {
        self.send(Command::SetVolumes(Volumes {
            master,
            effects,
            voice,
            ambient,
        }));
    }

    /// Play a one-shot. `priority`: higher wins when the per-sound or overall voice limit is
    /// reached (see [`crate::MAX_VOICES`], [`crate::PER_SOUND_LIMIT`]).
    pub fn play(&self, sound: SoundId, placement: Placement, priority: u8) {
        if self.shared.running.load(Ordering::Acquire) {
            self.send(Command::Play {
                sound,
                placement,
                priority,
            });
        }
    }

    /// Put a sound in loop slot `slot` (0…[`crate::LOOP_SLOTS`]) at `gain`, cross-fading from
    /// what the slot held; `None` fades the slot out. The same sound again only changes the gain.
    pub fn set_loop(&self, slot: usize, sound: Option<SoundId>, gain: f32) {
        self.send(Command::SetLoop { slot, sound, gain });
    }

    /// Silence every one-shot and loop.
    pub fn stop_all(&self) {
        self.send(Command::StopAll);
    }

    fn send(&self, command: Command) {
        if let Some(tx) = &self.commands {
            // The audio thread may have stopped (no device): the engine is then silent.
            let _ = tx.send(command);
        }
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        AudioEngine::new()
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop.take();
        self.commands.take();
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

/// The audio thread's body: everything raylib- and simulation-free.
fn run(commands: Receiver<Command>, stop: Receiver<()>, shared: Arc<Shared>) {
    let threads = std::thread::available_parallelism().map_or(2, |n| n.get().min(4));
    let bank = match SoundBank::decode_all(threads) {
        Ok(bank) => Arc::new(bank),
        Err(e) => {
            shared.set(EngineStatus::Silent {
                reason: format!("sound data: {e}"),
            });
            return;
        }
    };
    let host = cpal::default_host();
    let Some(device) = host.default_output_device() else {
        shared.set(EngineStatus::Silent {
            reason: "no output device".into(),
        });
        return;
    };
    let config = match device.default_output_config() {
        Ok(c) => c,
        Err(e) => {
            shared.set(EngineStatus::Silent {
                reason: format!("output configuration: {e}"),
            });
            return;
        }
    };
    let name = device.to_string();
    let rate = config.sample_rate();
    let stream = match config.sample_format() {
        SampleFormat::F32 => open::<f32>(&device, config.into(), &bank, commands, &shared),
        SampleFormat::I16 => open::<i16>(&device, config.into(), &bank, commands, &shared),
        SampleFormat::U16 => open::<u16>(&device, config.into(), &bank, commands, &shared),
        SampleFormat::I32 => open::<i32>(&device, config.into(), &bank, commands, &shared),
        SampleFormat::F64 => open::<f64>(&device, config.into(), &bank, commands, &shared),
        other => Err(format!("sample format {other} is not supported")),
    };
    match stream {
        Ok(stream) => {
            shared.set(EngineStatus::Running { device: name, rate });
            // Keep the stream alive until the engine is dropped.
            let _ = stop.recv();
            drop(stream);
        }
        Err(reason) => shared.set(EngineStatus::Silent { reason }),
    }
}

fn open<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    bank: &Arc<SoundBank>,
    commands: Receiver<Command>,
    shared: &Arc<Shared>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    if channels == 0 {
        return Err("the device has no output channels".into());
    }
    let mut mixer = Mixer::new(Arc::clone(bank), config.sample_rate);
    let mut stereo = vec![0.0f32; 4096];
    let counter = Arc::clone(shared);
    let stream = device
        .build_output_stream(
            config,
            move |out: &mut [T], _: &cpal::OutputCallbackInfo| {
                while let Ok(command) = commands.try_recv() {
                    mixer.apply(command);
                }
                for block in out.chunks_mut((stereo.len() / 2) * channels) {
                    let n = block.len() / channels;
                    let buf = &mut stereo[..n * 2];
                    mixer.render(buf);
                    for (frame, lr) in block.chunks_exact_mut(channels).zip(buf.as_chunks::<2>().0)
                    {
                        if channels == 1 {
                            frame[0] = T::from_sample(0.5 * (lr[0] + lr[1]));
                        } else {
                            frame[0] = T::from_sample(lr[0]);
                            frame[1] = T::from_sample(lr[1]);
                            for extra in &mut frame[2..] {
                                *extra = T::from_sample(0.0f32);
                            }
                        }
                    }
                    counter.frames.fetch_add(n as u64, Ordering::Relaxed);
                }
            },
            |e| eprintln!("audio: {e}"),
            None,
        )
        .map_err(|e| format!("output stream: {e}"))?;
    stream.play().map_err(|e| format!("start stream: {e}"))?;
    Ok(stream)
}
