use std::thread;
use std::time::Duration;

use utter_core::context::ContextMode;

fn main() {
    let delay = std::env::args()
        .nth(1)
        .and_then(|seconds| seconds.parse().ok())
        .unwrap_or(3);
    println!(
        "accessibility granted: {}",
        utter_os::permissions::accessibility_granted()
    );
    println!("focus the text field to inspect; reading in {delay} s");
    thread::sleep(Duration::from_secs(delay));
    println!("app pid: {:?}", utter_os::context::focused_app_pid());
    match serde_json::to_string_pretty(&utter_os::context::capture(ContextMode::Screen)) {
        Ok(json) => println!("{json}"),
        Err(error) => eprintln!("could not print the context: {error}"),
    }
}
