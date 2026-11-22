use utter_core::microphone::choose_microphone;

fn main() {
    let microphones = utter_os::microphones::list_microphones();
    for microphone in &microphones {
        println!(
            "{:<40} {:?}{}",
            microphone.name,
            microphone.connection,
            if microphone.is_default {
                "  (system default)"
            } else {
                ""
            }
        );
    }
    match choose_microphone(&microphones, None) {
        Some(chosen) => println!("automatic choice: {}", chosen.name),
        None => println!("no microphone found"),
    }
}
