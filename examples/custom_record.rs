use std::error;
use std::io::Write;
use std::time::Duration;
use std::{collections::VecDeque, fs::File};
use wasapi::*;

#[macro_use]
extern crate log;
use simplelog::*;

type Res<T> = Result<T, Box<dyn error::Error>>;

// Capture loop, capture samples and send in chunks of "chunksize" frames to channel
fn capture_loop(tx_capt: std::sync::mpsc::SyncSender<Vec<u8>>, chunksize: usize) -> Res<()> {
    let device = get_default_device(&Direction::Render)?;
    let mut audio_client = device.get_iaudioclient()?;

    let desired_format = WaveFormat::new(32, 32, &SampleType::Float, 44100, 2);

    let blockalign = desired_format.get_blockalign();
    debug!("Desired capture format: {:?}", desired_format);

    let (def_time, min_time) = audio_client.get_periods()?;
    debug!("default period {}, min period {}", def_time, min_time);

    audio_client.initialize_client(
        &desired_format,
        min_time as i64,
        &Direction::Capture,
        &ShareMode::Shared,
        true,
    )?;
    debug!("initialized capture");

    let h_event = audio_client.set_get_eventhandle()?;

    let buffer_frame_count = audio_client.get_bufferframecount()?;

    let render_client = audio_client.get_audiocaptureclient()?;
    let mut sample_queue: VecDeque<u8> = VecDeque::with_capacity(
        100 * blockalign as usize * (1024 + 2 * buffer_frame_count as usize),
    );
    audio_client.start_stream()?;
    loop {
        while sample_queue.len() > (blockalign as usize * chunksize as usize) {
            debug!("pushing samples");
            let mut chunk = vec![0u8; blockalign as usize * chunksize as usize];
            for element in chunk.iter_mut() {
                *element = sample_queue.pop_front().unwrap();
            }
            tx_capt.send(chunk)?;
        }
        trace!("capturing");
        render_client.read_from_device_to_deque(blockalign as usize, &mut sample_queue)?;
        if h_event.wait_for_event(1000000).is_err() {
            error!("error, stopping capture");
            audio_client.stop_stream()?;
            break;
        }
    }
    Ok(())
}

// Main loop
fn main() -> Res<()> {
    let _ = SimpleLogger::init(
        LevelFilter::Error,
        ConfigBuilder::new()
            .set_time_format_custom(format_description!("[hour]:[minute]:[second].[subsecond]"))
            .build(),
    );

    initialize_mta()?;

    let device = get_default_device(&Direction::Render)?;
    let mut audio_client = device.get_iaudioclient()?;
    let format = audio_client.get_mixformat().unwrap();

    // let desired_format = WaveFormat::new(32, 32, &SampleType::Float, 44100, 2);
    // let format = desired_format;
    println!("Format: {:?}", format);

    let blockalign = format.get_blockalign();

    let (def_time, min_time) = audio_client.get_periods()?;
    audio_client.initialize_client(
        &format,
        min_time as i64,
        &Direction::Capture,
        &ShareMode::Shared,
        true,
    )?;

    let buffer_frame_count = audio_client.get_bufferframecount()?;
    let render_client = audio_client.get_audiocaptureclient()?;
    let mut sample_queue: VecDeque<u8> = VecDeque::with_capacity(
        100 * blockalign as usize * (1024 + 2 * buffer_frame_count as usize),
    );
    let h_event = audio_client.set_get_eventhandle()?;

    let mut out_file = File::create("recorded.raw")?;

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    audio_client.start_stream()?;
    let time = std::time::Instant::now();
    while time.elapsed() < Duration::from_secs(10) {
        render_client.read_from_device_to_deque(blockalign as usize, &mut sample_queue)?;

        while sample_queue.len() > (blockalign as usize * buffer_frame_count as usize) {
            let mut chunk = vec![0u8; blockalign as usize * buffer_frame_count as usize];
            for element in chunk.iter_mut() {
                *element = sample_queue.pop_front().unwrap();
            }
            println!("Chunk: {:?}", chunk);
            out_file.write_all(&chunk)?;
        }
    }



    audio_client.stop_stream()?;

    Ok(())
}
