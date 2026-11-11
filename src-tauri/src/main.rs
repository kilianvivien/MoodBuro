#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    // Packaged-app check of the native timetable engine: converts a compiled-in
    // public fixture and compares it with InspCal's reviewed contract. Takes no
    // input, opens no window and touches no user data.
    if std::env::args().nth(1).as_deref() == Some("--inspcal-self-check") {
        std::process::exit(moodburo_lib::inspcal_self_check());
    }
    moodburo_lib::run();
}
