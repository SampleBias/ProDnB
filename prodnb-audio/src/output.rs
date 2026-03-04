use cpal::{Device, Stream, StreamConfig, SupportedStreamConfig, traits::{DeviceTrait, HostTrait, StreamTrait}};
use anyhow::{Result, Context};
use std::sync::Arc;
use super::synth::AudioEngine;

#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size: usize,
}

impl Default for AudioConfig {
    fn default() -> Self {
        AudioConfig {
            sample_rate: 44100,
            channels: 2,
            buffer_size: 512,
        }
    }
}

pub struct AudioOutput {
    stream: Stream,
    config: AudioConfig,
    is_playing: bool,
}

impl AudioOutput {
    pub fn new(engine: Arc<spin::Mutex<AudioEngine>>, _config: AudioConfig) -> Result<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .context("No default output device available")?;

        let supported_config = device.default_output_config()
            .context("Failed to get default output config")?;

        let stream_config = StreamConfig {
            channels: supported_config.channels(),
            sample_rate: cpal::SampleRate(44100),
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut engine = engine.lock();
                let samples = engine.render_block();
                let len = data.len().min(samples.len());
                data[..len].copy_from_slice(&samples[..len]);

                if len < data.len() {
                    data[len..].fill(0.0);
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;

        Ok(AudioOutput {
            stream,
            config: _config,
            is_playing: false,
        })
    }

    pub fn play(&mut self) -> Result<()> {
        if !self.is_playing {
            self.stream.play()
                .context("Failed to play audio stream")?;
            self.is_playing = true;
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if self.is_playing {
            self.stream.pause()
                .context("Failed to pause audio stream")?;
            self.is_playing = false;
        }
        Ok(())
    }
}
