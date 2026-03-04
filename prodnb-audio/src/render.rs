use std::fs::File;
use std::io::Write;
use anyhow::{Result, Context};
use super::synth::AudioEngine;
use prodnb_core::ArrangementPlan;

#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for RenderConfig {
    fn default() -> Self {
        RenderConfig {
            sample_rate: 44100,
            channels: 2,
        }
    }
}

pub struct WavRenderer;

impl WavRenderer {
    pub fn render_to_file(
        engine: &mut AudioEngine,
        arrangement: &ArrangementPlan,
        output_path: &str,
        config: RenderConfig,
    ) -> Result<()> {
        let total_bars: u16 = arrangement.sections.iter()
            .map(|s| s.bars)
            .sum();

        let samples_per_beat = config.sample_rate as f64 * 60.0 / arrangement.bpm as f64;
        let total_samples = (total_bars * 4) as u64 * samples_per_beat as u64;
        let total_samples = total_samples / 4 * 4;

        let mut file = File::create(output_path)
            .context("Failed to create output file")?;

        let header = Self::build_wav_header(total_samples, config);
        file.write_all(&header)?;

        let chunk_size = 4096;
        let mut total_rendered = 0u64;

        while total_rendered < total_samples {
            let samples_to_render = chunk_size.min((total_samples - total_rendered) as usize);

            let mut buffer = vec![0f32; samples_to_render];
            let mut rendered = 0;

            while rendered < samples_to_render {
                let block = engine.render_block();
                let copy_len = (block.len()).min(samples_to_render - rendered);
                buffer[rendered..rendered + copy_len].copy_from_slice(&block[..copy_len]);
                rendered += copy_len;
            }

            let i16_buffer: Vec<i16> = buffer.iter()
                .map(|&s| (s * 32767.0) as i16)
                .collect();

            for sample in i16_buffer {
                file.write_all(&sample.to_le_bytes())?;
            }

            total_rendered += samples_to_render as u64;
        }

        Ok(())
    }

    fn build_wav_header(total_samples: u64, config: RenderConfig) -> [u8; 44] {
        let bytes_per_sample = 2;
        let block_align = config.channels as u32 * bytes_per_sample;
        let byte_rate = config.sample_rate as u32 * block_align;
        let data_size = total_samples * block_align as u64;
        let file_size = 36 + data_size;

        let mut header = [0u8; 44];

        header[0..4].copy_from_slice(b"RIFF");
        header[4..8].copy_from_slice(&(file_size as u32).to_le_bytes());
        header[8..12].copy_from_slice(b"WAVE");
        header[12..16].copy_from_slice(b"fmt ");
        header[16..20].copy_from_slice(&16u32.to_le_bytes());
        header[20..22].copy_from_slice(&1u16.to_le_bytes());
        header[22..24].copy_from_slice(&(config.channels).to_le_bytes());
        header[24..28].copy_from_slice(&(config.sample_rate).to_le_bytes());
        header[28..32].copy_from_slice(&byte_rate.to_le_bytes());
        header[32..34].copy_from_slice(&block_align.to_le_bytes());
        header[34..36].copy_from_slice(&bytes_per_sample.to_le_bytes());
        header[36..40].copy_from_slice(b"data");
        header[40..44].copy_from_slice(&(data_size as u32).to_le_bytes());

        header
    }
}
