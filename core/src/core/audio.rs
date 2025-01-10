use std::sync::Arc;

use cpal::{
    self,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use tokio::time::{Duration, Sleep};
use webrtc::{
    media::Sample,
    track::track_local::{track_local_static_sample::TrackLocalStaticSample, TrackLocal},
};

const SAMPLE_RATE: u32 = 480000;
const CHANNELS: u16 = 2;

struct Audio {
    host: cpal::Host,
    device: cpal::Device,
}

impl Audio {
    fn new() -> Self {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .expect("Failed to find input device");
        Self { host, device }
    }

    async fn stream_audio(&self, track: Arc<TrackLocalStaticSample>) {
        let track = Arc::clone(&track);
        let config = self.device.default_input_config().unwrap();

        let stream = self.device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let sample = Sample {
                    data: vec![].into(),
                    duration: Duration::from_secs_f32(1.0 / 50.0),
                    ..Default::default()
                };
                track.write_sample(&sample);
            },
            |err| eprintln!("Error capturing audio: {:?}", err),
        );
    }

    /// Capture microphone input and send it to a WebRTC audio track.
    async fn capture_and_stream_audio(&self, track: Arc<TrackLocalStaticSample>) {
        let track = Arc::clone(&track);
        let config = self.device.default_input_config().unwrap();

        let track_clone = track.clone();

        // Build and run the audio capture stream
        let stream = self
            .device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Convert captured audio to bytes and send to the WebRTC track
                    let audio_data: Vec<u8> = data
                        .iter()
                        .flat_map(|&sample| sample.to_le_bytes().to_vec())
                        .collect();

                    let track_clone = Arc::clone(&track);

                    let _ = tokio::spawn(async move {
                        let sample = Sample {
                            data: audio_data.clone().into(),
                            duration: Duration::from_secs_f32(1.0 / 50.0),
                            ..Default::default()
                        };

                        if let Err(err) = track_clone.write_sample(&sample).await {
                            eprintln!("Error sending audio sample: {:?}", err);
                        }
                    });
                },
                |err| eprintln!("Error capturing audio: {:?}", err),
            )
            .expect("Failed to build input stream");

        stream.play().expect("Error playing stream");
    }
}
