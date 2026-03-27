use bms::model::layer::Sequence;
use rubato_render::color::Rectangle;
use rubato_render::texture::TextureRegion;

use super::{BGAProcessor, BgaRenderType, BgaRenderer};
use crate::play::skin::bga::{StretchType, StretchTypeExt};

impl BGAProcessor {
    /// Draw BGA content to the given renderer.
    /// Translated from: Java BGAProcessor.drawBGA(SkinBGA dst, SkinObjectRenderer sprite, Rectangle r)
    ///
    /// The `stretch` parameter comes from SkinBGA.stretch_type().
    /// The `color` and `blend` are from the SkinObject destination state.
    pub fn draw_bga(
        &mut self,
        renderer: &mut dyn BgaRenderer,
        r: &Rectangle,
        stretch: StretchType,
        color: (f32, f32, f32, f32),
        blend: i32,
    ) {
        renderer.set_color_rgba(color.0, color.1, color.2, color.3);
        renderer.set_blend(blend);

        if self.time < 0 {
            // Blank screen before playback starts
            let blank_region = TextureRegion::from_texture(self.blanktex.clone());
            renderer.draw(&blank_region, r.x, r.y, r.width, r.height);
            return;
        }

        if self.misslayer.is_some()
            && self.misslayertime != 0
            && self.time >= self.misslayertime
            && self.time < self.misslayertime + self.get_misslayer_duration
        {
            // Draw miss layer
            let miss_index = self.miss_layer_index();
            if miss_index != Sequence::END {
                let miss = self.bga_data(self.time, miss_index, true);
                if let Some(tex) = miss {
                    renderer.set_type(BgaRenderType::Linear);
                    self.draw_bga_fix_ratio(renderer, r, &tex, stretch);
                }
            }
        } else {
            // Draw BGA
            let bga_id = self.playingbgaid;
            let rbga = self.rbga;
            let bga_tex = self.bga_data(self.time, bga_id, rbga);
            self.rbga = true;
            if let Some(tex) = bga_tex {
                let is_movie = self.is_movie(bga_id);
                if is_movie {
                    renderer.set_type(BgaRenderType::Ffmpeg);
                } else {
                    renderer.set_type(BgaRenderType::Linear);
                }
                self.draw_bga_fix_ratio(renderer, r, &tex, stretch);
            } else {
                let blank_region = TextureRegion::from_texture(self.blanktex.clone());
                renderer.draw(&blank_region, r.x, r.y, r.width, r.height);
            }

            // Draw layer
            let layer_id = self.playinglayerid;
            let rlayer = self.rlayer;
            let layer_tex = self.bga_data(self.time, layer_id, rlayer);
            self.rlayer = true;
            if let Some(tex) = layer_tex {
                let is_movie = self.is_movie(layer_id);
                if is_movie {
                    renderer.set_type(BgaRenderType::Ffmpeg);
                } else {
                    renderer.set_type(BgaRenderType::Layer);
                }
                self.draw_bga_fix_ratio(renderer, r, &tex, stretch);
            }
        }
    }

    /// Get the BGA id from the miss layer sequence for the current time.
    /// Returns Sequence::END if no valid index.
    pub(super) fn miss_layer_index(&self) -> i32 {
        if let Some(ref misslayer) = self.misslayer
            && !misslayer.sequence.is_empty()
            && !misslayer.sequence[0].is_empty()
        {
            let seq = &misslayer.sequence[0];
            let elapsed = self.time - self.misslayertime;
            let duration = self.get_misslayer_duration;
            if duration > 0 {
                let idx =
                    ((seq.len() as i64 - 1) * elapsed / duration).clamp(0, seq.len() as i64 - 1);
                return seq[idx as usize].id;
            }
        }
        Sequence::END
    }

    /// Draw BGA with aspect-ratio correction.
    /// Translated from: Java BGAProcessor.drawBGAFixRatio(SkinBGA dst, SkinObjectRenderer sprite, Rectangle r, Texture bga)
    fn draw_bga_fix_ratio(
        &mut self,
        renderer: &mut dyn BgaRenderer,
        r: &Rectangle,
        bga: &rubato_render::texture::Texture,
        stretch: StretchType,
    ) {
        self.tmp_rect.set(r);
        self.image.set_texture(bga.clone());
        self.image.set_region_from(0, 0, bga.width, bga.height);

        // Apply stretch type to modify rectangle and image region
        stretch.stretch_rect(&mut self.tmp_rect, &mut self.image);

        renderer.draw(
            &self.image,
            self.tmp_rect.x,
            self.tmp_rect.y,
            self.tmp_rect.width,
            self.tmp_rect.height,
        );
    }
}
