use std::error;
use std::io::Write;
use std::time::Duration;
use std::{collections::VecDeque, fs::File};
use wasapi::*;

#[macro_use]
extern crate log;
use simplelog::*;

use hound::{SampleFormat, WavWriter};

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

    let mut out_file = File::create("/files/recorded.raw")?;

    // let spec = hound::WavSpec {
    //     channels: 2,
    //     sample_rate: 44100,
    //     bits_per_sample: 32,
    //     sample_format: hound::SampleFormat::Float,
    // };
    let h_event = audio_client.set_get_eventhandle()?;

    let spec = hound::WavSpec {
        channels: format.get_nchannels(),
        sample_rate: format.get_samplespersec(),
        bits_per_sample: format.get_bitspersample(),
        sample_format: hound::SampleFormat::Float,
    };

    println!("Spec: {:?}", spec);

    let mut buff = Vec::new();

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

    let mut writer = WavWriter::create("/files/recorded.wav", spec).unwrap();
    for i in 0..(buff.len() / 4) {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&buff[i * 4..(i + 1) * 4]);
        let f = f32::from_le_bytes(bytes);
        writer.write_sample(f).unwrap();
    }
    Ok(())
}
