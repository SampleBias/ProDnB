use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Widget, Borders, Block},
    style::{Color, Style},
};
use std::convert::identity;

pub struct Oscilloscope<'a> {
    samples: &'a [f32],
}

impl<'a> Oscilloscope<'a> {
    pub fn new(samples: &'a [f32]) -> Self {
        Oscilloscope { samples }
    }
}

impl Widget for Oscilloscope<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Oscilloscope ")
            .title_style(Style::default().fg(Color::Green));

        let inner = block.inner(area);
        block.render(area, buf);

        if self.samples.is_empty() || inner.height < 2 {
            return;
        }

        let width = inner.width as usize;
        let height = inner.height as usize;
        let center_y = inner.y + (height / 2) as u16;

        let samples_per_pixel = self.samples.len().saturating_sub(1) as f64 / width as f64;

        for x in 0..width {
            let sample_idx = (x as f64 * samples_per_pixel).floor() as usize;
            if sample_idx >= self.samples.len() {
                continue;
            }

            let sample = self.samples[sample_idx];
            let offset = ((sample as f64) * (height as f64 / 2.0)).floor() as i32;
            let y = (center_y as i32 - offset).clamp(0, (height - 1) as i32) as u16;

            let symbol = match sample.abs() {
                v if v > 0.8 => "█",
                v if v > 0.6 => "▓",
                v if v > 0.4 => "▒",
                v if v > 0.2 => "░",
                _ => "┼",
            };

            buf.set_string(
                inner.x + x as u16,
                y,
                symbol,
                Style::default().fg(Color::Cyan),
            );

            if offset != 0 {
                for dy in 1..=offset.abs() as usize {
                    if dy >= height / 2 {
                        break;
                    }
                    let draw_y = if offset > 0 {
                        center_y - dy as u16
                    } else {
                        center_y + dy as u16
                    };
                    if draw_y >= inner.y && draw_y < inner.y + inner.height {
                        buf.set_string(
                            inner.x + x as u16,
                            draw_y,
                            "│",
                            Style::default().fg(Color::DarkGray),
                        );
                    }
                }
            }
        }

        for y in inner.top()..inner.bottom() {
            buf.set_string(
                inner.x + width as u16 / 2,
                y,
                "─",
                Style::default().fg(Color::DarkGray),
            );
        }
    }
}

pub struct Spectrum<'a> {
    samples: &'a [f32],
    bands: usize,
}

impl<'a> Spectrum<'a> {
    pub fn new(samples: &'a [f32]) -> Self {
        Spectrum {
            samples,
            bands: 32,
        }
    }

    pub fn bands(mut self, bands: usize) -> Self {
        self.bands = bands;
        self
    }
}

impl Widget for Spectrum<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Spectrum ")
            .title_style(Style::default().fg(Color::Magenta));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 2 || inner.width < 2 {
            return;
        }

        let bands = self.bands.min(inner.width as usize);
        let band_width = inner.width as usize / bands;
        let height = inner.height as usize - 1;

        for band in 0..bands {
            let magnitude = self.simplify_magnitude(band, bands);
            let bars = (magnitude * height as f32).floor() as usize;
            let bars = bars.min(height);

            for i in 0..bars {
                let y = inner.y + height as u16 - 1 - i as u16;
                let x = inner.x + (band * band_width) as u16;

                let symbol = if i == bars.saturating_sub(1) {
                    "▀"
                } else if i > bars / 2 {
                    "█"
                } else {
                    "▓"
                };

                let color = self.frequency_color(band, bands);
                buf.set_string(x, y, symbol, Style::default().fg(color));
            }
        }
    }
}

impl Spectrum<'_> {
    fn simplify_magnitude(&self, band: usize, total_bands: usize) -> f32 {
        let start = (band * self.samples.len() / total_bands) as usize;
        let end = ((band + 1) * self.samples.len() / total_bands) as usize;
        let end = end.min(self.samples.len());

        if start >= end {
            return 0.0;
        }

        let sum: f32 = self.samples[start..end]
            .iter()
            .map(|s| s.abs())
            .sum();

        (sum / (end - start) as f32) * 2.0
    }

    fn frequency_color(&self, band: usize, total: usize) -> Color {
        let ratio = band as f32 / total as f32;
        match ratio {
            r if r < 0.2 => Color::Red,
            r if r < 0.4 => Color::Yellow,
            r if r < 0.6 => Color::Green,
            r if r < 0.8 => Color::Cyan,
            _ => Color::Blue,
        }
    }
}

pub struct Vectorscope<'a> {
    samples: &'a [f32],
}

impl<'a> Vectorscope<'a> {
    pub fn new(samples: &'a [f32]) -> Self {
        Vectorscope { samples }
    }
}

impl Widget for Vectorscope<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Vectorscope ")
            .title_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 2 || inner.width < 2 {
            return;
        }

        let center_x = inner.x + inner.width / 2;
        let center_y = inner.y + inner.height / 2;
        let radius = (inner.width.min(inner.height) / 2).saturating_sub(1) as i32;

        for angle in 0..360 {
            let rad = angle as f64 * std::f64::consts::PI / 180.0;
            let x = center_x as i32 + (radius as f64 * rad.cos()).round() as i32;
            let y = center_y as i32 - (radius as f64 * rad.sin()).round() as i32;

            let symbol = if angle % 45 == 0 {
                "+"
            } else if angle % 90 == 0 {
                "─"
            } else {
                "·"
            };

            buf.set_string(
                x as u16,
                y as u16,
                symbol,
                Style::default().fg(Color::DarkGray),
            );
        }

        for axis in [0i32, 45, 90, 135] {
            let rad = axis as f64 * std::f64::consts::PI / 180.0;
            let x = center_x as i32 + (radius as f64 * rad.cos()).round() as i32;
            let y = center_y as i32 - (radius as f64 * rad.sin()).round() as i32;

            buf.set_string(
                center_x,
                center_y,
                "┼",
                Style::default().fg(Color::DarkGray),
            );

            let dx = (x - center_x as i32).signum();
            let dy = (y - center_y as i32).signum();

            for i in 1..=radius {
                let px = center_x as i32 + dx * i;
                let py = center_y as i32 + dy * i;

                let symbol = if dx != 0 && dy != 0 {
                    "╱"
                } else if dx != 0 {
                    "─"
                } else {
                    "│"
                };

                buf.set_string(
                    px as u16,
                    py as u16,
                    symbol,
                    Style::default().fg(Color::DarkGray),
                );
            }
        }

        for i in (0..self.samples.len()).step_by(2) {
            if i + 1 >= self.samples.len() {
                break;
            }

            let left = self.samples[i];
            let right = self.samples[i + 1];

            let x = center_x as i32 + (radius as f32 * left).round() as i32;
            let y = center_y as i32 - (radius as f32 * right).round() as i32;

            if x >= center_x as i32 - radius && x <= center_x as i32 + radius
                && y >= center_y as i32 - radius && y <= center_y as i32 + radius
            {
                buf.set_string(
                    x as u16,
                    y as u16,
                    "•",
                    Style::default().fg(Color::Green),
                );
            }
        }
    }
}
