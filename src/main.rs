use std::env;
use std::process;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use bard::app::InterruptFlag;

static INTERRUPT: AtomicBool = AtomicBool::new(false);

fn main() {
    let args: Vec<_> = env::args_os().collect();

    ctrlc::set_handler(|| {
        INTERRUPT.store(true, Ordering::Relaxed);
        // TODO: refactor ctrlc such that a thread is not needed for just a flag.
    })
    .expect("Could not set up interrupt handler.");

    process::exit(bard::bard(&args[..], InterruptFlag(&INTERRUPT)));
}
