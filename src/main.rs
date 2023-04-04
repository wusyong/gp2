/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! The `servo` test application.
//!
//! Creates a `Servo` instance with a simple implementation of
//! the compositor's `WindowMethods` to create a working web browser.
//!
//! This browser's implementation of `WindowMethods` is built on top
//! of [winit], the cross-platform windowing library.
//!
//! For the engine itself look next door in `components/servo/lib.rs`.
//!
//! [winit]: https://github.com/rust-windowing/winit

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[macro_use]
extern crate sig;

mod app;
mod backtrace;
mod browser;
mod crash_handler;
mod embedder;
mod events_loop;
mod headed_window;
mod headless_window;
mod keyutils;
mod prefs;
mod resources;
mod window_trait;

use app::App;
use getopts::Options;
use servo::config::opts::{self, ArgumentParsingResult};
use servo::servo_config::pref;
use std::env;
use std::process;

pub fn main() {
    crash_handler::install();

    resources::init();

    // Parse the command line options and store them globally
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.optflag(
        "",
        "angle",
        "Use ANGLE to create a GL context (Windows-only)",
    );
    opts.optflag("b", "no-native-titlebar", "Do not use native titlebar");
    opts.optopt("", "device-pixel-ratio", "Device pixels per px", "");
    opts.optopt(
        "u",
        "user-agent",
        "Set custom user agent string (or ios / android / desktop for platform default)",
        "NCSA Mosaic/1.0 (X11;SunOS 4.1.4 sun4m)",
    );
    opts.optmulti(
        "",
        "pref",
        "A preference to set to enable",
        "dom.bluetooth.enabled",
    );
    opts.optmulti(
        "",
        "pref",
        "A preference to set to disable",
        "dom.webgpu.enabled=false",
    );

    let opts_matches;
    let content_process_token;

    match opts::from_cmdline_args(opts, &args) {
        ArgumentParsingResult::ContentProcess(matches, token) => {
            opts_matches = matches;
            content_process_token = Some(token);
            if opts::get().is_running_problem_test && env::var("RUST_LOG").is_err() {
                env::set_var("RUST_LOG", "compositing::constellation");
            }
        },
        ArgumentParsingResult::ChromeProcess(matches) => {
            opts_matches = matches;
            content_process_token = None;
        },
    };

    prefs::register_user_prefs(&opts_matches);

    log_panics::init();

    if let Some(token) = content_process_token {
        return servo::run_content_process(token);
    }

    if opts::get().is_printing_version {
        println!("{}", servo_version());
        process::exit(0);
    }

    let do_not_use_native_titlebar =
        opts_matches.opt_present("no-native-titlebar") || !(pref!(shell.native_titlebar.enabled));
    let device_pixels_per_px = opts_matches.opt_str("device-pixel-ratio").map(|dppx_str| {
        dppx_str.parse().unwrap_or_else(|err| {
            error!("Error parsing option: --device-pixel-ratio ({})", err);
            process::exit(1);
        })
    });

    let user_agent = opts_matches.opt_str("u");

    App::run(do_not_use_native_titlebar, device_pixels_per_px, user_agent);
}

pub fn servo_version() -> String {
    let cargo_version = env!("CARGO_PKG_VERSION");
    let git_info = option_env!("GIT_INFO");
    match git_info {
        Some(info) => format!("Servo {}{}", cargo_version, info),
        None => format!("Servo {}", cargo_version),
    }
}
