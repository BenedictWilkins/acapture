// This program is just a testing application
// Refer to `lib.rs` for the library source code

use scap::{
    capturer::{Capturer, Options},
    frame::Frame,
};

fn main() {
    // Check if the platform is supported
    if !scap::is_supported() {
        println!("❌ Platform not supported");
        return;
    }

    // Check if we have permission to capture screen
    // If we don't, request it.
    if !scap::has_permission() {
        println!("❌ Permission not granted. Requesting permission...");
        if !scap::request_permission() {
            println!("❌ Permission denied");
            return;
        }
    }

    // Get recording targets
    let mut targets = scap::get_all_targets();
    for target in &targets {
        println!("target: {:?}", target)
    }
    let target = targets.remove(1);
    // Create Options
    let options = Options {
        fps: 30,
        target: Some(target),
        show_cursor: true,
        show_highlight: true,
        excluded_targets: None,
        ..Default::default()
    };

    // Create Recorder with options
    let mut recorder = Capturer::new(options);

    // Start Capture
    recorder.start_capture();
    println!("Frame output size: {:?}", recorder.get_output_frame_size());

    // Capture 100 frames
    let mut start_time: u64 = 0;
    for i in 0..100 {
        let frame = recorder.get_next_frame().expect("Error");
        (match frame {
            Frame::BGRA(frame) => {
                if start_time == 0 {
                    start_time = frame.display_time;
                }
                println!(
                    "Recieved BGRA frame {} of width {} and height {} and time {}",
                    i,
                    frame.width,
                    frame.height,
                    frame.display_time - start_time
                );
                Ok(())
            }
            _ => Err(format!("Recived invalid frame type: {:?}", frame)),
        })
        .expect("Error");
    }
    recorder.stop_capture();
}
