use std::collections::VecDeque;
use std::error;
use std::time::Duration;
use wasapi::*;

extern crate log;

use hound::WavWriter;

type Res<T> = Result<T, Box<dyn error::Error>>;



fn main() -> Res<()> {
    initialize_mta()?;

    let device = get_default_device(&Direction::Render)?;
    let mut audio_client = device.get_iaudioclient()?;

    // let desired_format = audio_client.get_mixformat().unwrap();
    let desired_format = WaveFormat::new(32, 32, &SampleType::Float, 44100, 2);
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

    let _ = audio_client.set_get_eventhandle()?;

    // ------------- WAV Config ----------------
    let spec = hound::WavSpec {
        channels: format.get_nchannels(),
        sample_rate: format.get_samplespersec(),
        bits_per_sample: format.get_bitspersample(),
        sample_format: hound::SampleFormat::Float,
    };
    println!("Spec: {:?}", spec);
    let mut writer = WavWriter::create("files\\recorded.wav", spec).unwrap();

    // ------------- Stream Capture ----------------
    println!("Starting stream");
    audio_client.start_stream()?;
    let time = std::time::Instant::now();
    while time.elapsed() < Duration::from_secs(10) {
        render_client.read_from_device_to_deque(blockalign as usize, &mut sample_queue)?;
        // Sampling can be empty sometimes, so we need to check for that
        while sample_queue.len() > (blockalign as usize * buffer_frame_count as usize) {
            let mut chunk = vec![0u8; blockalign as usize * buffer_frame_count as usize];
            for element in chunk.iter_mut() {
                *element = sample_queue.pop_front().unwrap();
            }

            for i in 0..(chunk.len() / 4) {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&chunk[i * 4..(i + 1) * 4]);
                let f = f32::from_le_bytes(bytes);
                writer.write_sample(f).unwrap();
            }
        }
    }
    audio_client.stop_stream()?;

    println!("Done");
    Ok(())
}
