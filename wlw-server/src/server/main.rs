#[macro_use]
extern crate log;
use wintrap::{self, Signal};
pub mod context;
#[cfg(debug_assertions)]
pub mod debug;
pub mod hookevent;
pub mod hookmanager;
pub mod pipeserver;
use crate::context::Context;
use crossbeam_channel as xchan;

use flexi_logger::Logger;

fn run() -> Result<(), context::Error> {
    let (event_sender, event_receiver) = xchan::unbounded::<context::Event>();
    let interrupt_event_sender = event_sender.clone();
    let mut context = Context::new(event_sender, event_receiver)?;
    wintrap::trap(
        &[Signal::CtrlC, Signal::CloseWindow, Signal::CloseConsole],
        move |_| {
            interrupt_event_sender
                .send(context::Event::Interrupt)
                .unwrap()
        },
        move || context.run(),
    )
    .unwrap()?;
    Ok(())
}

fn main() {
    Logger::with_env_or_str("trace")
        .format(|w, record| {
            write!(
                w,
                "SERVER:{} [{}] {}",
                record.level(),
                record.module_path().unwrap_or("<unnamed>"),
                record.args()
            )
        })
        .start()
        .unwrap();
    match run() {
        Ok(_) => {}
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    }
}
