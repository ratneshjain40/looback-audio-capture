use std::error;
use std::io::Write;
use std::time::Duration;
use std::{collections::VecDeque, fs::File};
use wasapi::*;

extern crate log;
use simplelog::*;

use hound::WavWriter;

type Res<T> = Result<T, Box<dyn error::Error>>;
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

    // let format = audio_client.get_mixformat().unwrap();
    let desired_format = WaveFormat::new(16, 16, &SampleType::Int, 44100, 2);
    let format = desired_format;
    println!("Format: {:?}", format);

    let blockalign = format.get_blockalign();

    let (_, min_time) = audio_client.get_periods()?;
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

    let mut out_file = File::create("files\\recorded.raw")?;

    let _ = audio_client.set_get_eventhandle()?;

    let mut buff = Vec::new();

    println!("Starting stream");
    audio_client.start_stream()?;
    let time = std::time::Instant::now();
    while time.elapsed() < Duration::from_secs(10) {
        render_client.read_from_device_to_deque(blockalign as usize, &mut sample_queue)?;

        while sample_queue.len() > (blockalign as usize * buffer_frame_count as usize) {
            let mut chunk = vec![0u8; blockalign as usize * buffer_frame_count as usize];
            for element in chunk.iter_mut() {
                *element = sample_queue.pop_front().unwrap();
            }
            out_file.write_all(&chunk)?;
            for element in chunk {
                buff.push(element);
            }
        }
    }
    audio_client.stop_stream()?;

    println!("Writing to file... ");
    let spec = hound::WavSpec {
        channels: format.get_nchannels(),
        sample_rate: format.get_samplespersec(),
        bits_per_sample: format.get_bitspersample(),
        sample_format: hound::SampleFormat::Int,
    };

    println!("Spec: {:?}", spec);
    let mut writer = WavWriter::create("files\\recorded.wav", spec).unwrap();
    for i in 0..(buff.len() / 2) {
        let mut bytes = [0u8; 2];
        bytes.copy_from_slice(&buff[i * 2..(i + 1) * 2]);
        let f = i16::from_le_bytes(bytes);
        writer.write_sample(f).unwrap();
    }
    // for i in 0..(buff.len() / 4) {
    //     let mut bytes = [0u8; 4];
    //     bytes.copy_from_slice(&buff[i * 4..(i + 1) * 4]);
    //     let f = f32::from_le_bytes(bytes);
    //     writer.write_sample(f).unwrap();
    // }
    println!("Done");
    Ok(())
}
