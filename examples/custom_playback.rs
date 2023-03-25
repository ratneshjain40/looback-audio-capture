use std::error;
use std::io::Read;
use std::{collections::VecDeque, fs::File};
use wasapi::*;

#[macro_use]
extern crate log;

type Res<T> = Result<T, Box<dyn error::Error>>;

// Main loop
fn main() -> Res<()> {
    initialize_mta()?;

    let device = get_default_device(&Direction::Render)?;
    let mut audio_client = device.get_iaudioclient()?;

    let desired_format = audio_client.get_mixformat().unwrap();
    // let desired_format = WaveFormat::new(32, 32, &SampleType::Float, 44100, 2);

    let blockalign = desired_format.get_blockalign();
    debug!("Desired playback format: {:?}", desired_format);

    let (def_time, min_time) = audio_client.get_periods()?;
    debug!("default period {}, min period {}", def_time, min_time);

    audio_client.initialize_client(
        &desired_format,
        min_time as i64,
        &Direction::Render,
        &ShareMode::Shared,
        true,
    )?;
    debug!("initialized playback");

    let h_event = audio_client.set_get_eventhandle()?;

    let mut buffer_frame_count = audio_client.get_bufferframecount()?;

    let render_client = audio_client.get_audiorenderclient()?;
    let mut sample_queue: VecDeque<u8> = VecDeque::with_capacity(
        100 * blockalign as usize * (1024 + 2 * buffer_frame_count as usize),
    );

    let mut file = File::open("recorded.raw")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    println!("buffer len {}", buffer.len());
    // Convert buffer to VecDeque
    let mut buffer_queue: VecDeque<u8> = VecDeque::with_capacity(buffer.len());
    for element in buffer.iter() {
        buffer_queue.push_back(*element);
    }
    println!("buffer_queue len {}", buffer_queue.len());

    audio_client.start_stream()?;
    loop {
        buffer_frame_count = audio_client.get_available_space_in_frames()?;
        trace!("New buffer frame count {}", buffer_frame_count);
        while sample_queue.len() < (blockalign as usize * buffer_frame_count as usize) {
            match buffer_queue.pop_front() {
                Some(x) => sample_queue.push_back(x),
                None => {
                    break;
                }
            }
        }

        render_client.write_to_device_from_deque(
            buffer_frame_count as usize,
            blockalign as usize,
            &mut sample_queue,
            None,
        )?;

        if h_event.wait_for_event(10000).is_err() {
            error!("error, stopping playback");
            audio_client.stop_stream()?;
            break;
        }
    }

    Ok(())
}


