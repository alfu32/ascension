//! Application start-up: render thread, GPUI window, clean shutdown.

use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use gpui::{
    App, AppContext, Application, Bounds, Entity, TitlebarOptions, Window, WindowBounds,
    WindowDecorations, WindowOptions, point, px, size,
};
use gpui_component::Root;
use rom_render::{RenderBridge, RenderWorker, ShutdownHandle};

use crate::args::Options;
use crate::bench;
use crate::world_view::WorldView;

/// Opens the window and runs until it is closed (or the benchmark finishes).
pub fn run(options: &Options) -> ExitCode {
    // GPUI unwraps platform initialisation errors; report them instead of a raw panic.
    let application = match std::panic::catch_unwind(Application::new) {
        Ok(application) => application,
        Err(_) => {
            eprintln!("rise-of-man: could not initialise the windowing system");
            return ExitCode::FAILURE;
        }
    };
    let (mut worker, bridge) = match RenderWorker::start() {
        Ok(started) => started,
        Err(error) => {
            eprintln!("rise-of-man: could not start the render thread: {error}");
            return ExitCode::FAILURE;
        }
    };
    bridge.publish_render_scale(options.scale);
    let shutdown = worker.shutdown_handle();
    let exit_code = Arc::new(AtomicU8::new(0));
    let app_exit_code = Arc::clone(&exit_code);
    let options = options.clone();

    application.run(move |cx: &mut App| {
        gpui_component::init(cx);
        let (width, height) = options.size;
        let canvas = size(px(width as f32), px(height as f32));
        let mut window_options = WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("Rise of Man".into()),
                ..Default::default()
            }),
            app_id: Some("rise-of-man".into()),
            ..Default::default()
        };
        let opened = match options.bench_seconds {
            // The benchmark window has no title bar or component frame (which pads client-decorated
            // windows for a shadow), so the whole requested size is canvas even at monitor size.
            Some(seconds) => {
                window_options.window_bounds = Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.0), px(0.0)),
                    canvas,
                )));
                window_options.window_decorations = Some(WindowDecorations::Client);
                let bench = bench::Bench {
                    seconds,
                    requested: options.size,
                    scale: options.scale,
                };
                let exit_code = Arc::clone(&app_exit_code);
                cx.open_window(window_options, move |window, cx| {
                    let view = world_view(&bridge, &shutdown, options.size, window, cx);
                    bench::start(bench, view.clone(), bridge, shutdown, exit_code, cx);
                    view
                })
                .map(|_| ())
            }
            None => {
                window_options.window_bounds = Some(WindowBounds::centered(canvas, cx));
                cx.open_window(window_options, move |window, cx| {
                    let view = world_view(&bridge, &shutdown, options.size, window, cx);
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .map(|_| ())
            }
        };
        if let Err(error) = opened {
            eprintln!("rise-of-man: could not open the window: {error:#}");
            app_exit_code.store(1, Ordering::Relaxed);
            cx.spawn(async |cx| {
                let _ = cx.update(|cx| cx.quit());
            })
            .detach();
            return;
        }
        cx.activate(true);
    });
    worker.stop();
    ExitCode::from(exit_code.load(Ordering::Relaxed))
}

/// Sets up the freshly opened window and creates the world view for it.
fn world_view(
    bridge: &RenderBridge,
    shutdown: &ShutdownHandle,
    (width, height): (u32, u32),
    window: &mut Window,
    cx: &mut App,
) -> Entity<WorldView> {
    let close_shutdown = shutdown.clone();
    // GPUI's X11 client holds its state borrowed while asking this, so calling `cx.quit()`
    // here panics; the event loop ends by itself once the last window is gone.
    window.on_window_should_close(cx, move |_, _| {
        close_shutdown.stop_and_wait();
        true
    });
    // The requested size is in physical pixels; GPUI sizes windows in logical ones.
    let scale_factor = window.scale_factor();
    if scale_factor != 1.0 {
        window.resize(size(
            px(width as f32 / scale_factor),
            px(height as f32 / scale_factor),
        ));
    }
    let bridge = bridge.clone();
    cx.new(|cx| WorldView::new(bridge, window, cx))
}
