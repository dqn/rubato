// Draw methods for SkinObjectData.
// Renders prepared skin objects to a SkinObjectRenderer (sprite batch).

use crate::reexports::TextureRegion;

use super::renderer::{DrawRotatedParams, SkinObjectRenderer};
use super::{DrawImageAtParams, SkinObjectData};

impl SkinObjectData {
    pub fn draw_image(&mut self, sprite: &mut SkinObjectRenderer, image: &TextureRegion) {
        if self.color.a == 0.0 {
            return;
        }

        self.tmp_rect.set(&self.region);
        self.stretch
            .stretch_rect(&mut self.tmp_rect, &mut self.tmp_image, image);
        sprite.set_color(&self.color);
        sprite.blend = self.dstblend;
        sprite.obj_type =
            if self.dstfilter != 0 && self.image_type == SkinObjectRenderer::TYPE_NORMAL {
                if self.tmp_rect.width == self.tmp_image.region_width as f32
                    && self.tmp_rect.height == self.tmp_image.region_height as f32
                {
                    SkinObjectRenderer::TYPE_NORMAL
                } else {
                    SkinObjectRenderer::TYPE_BILINEAR
                }
            } else {
                self.image_type
            };

        if self.angle != 0 {
            sprite.draw_rotated(DrawRotatedParams {
                image: &self.tmp_image,
                x: self.tmp_rect.x,
                y: self.tmp_rect.y,
                w: self.tmp_rect.width,
                h: self.tmp_rect.height,
                cx: self.centerx,
                cy: self.centery,
                angle: self.angle,
            });
        } else {
            sprite.draw(
                &self.tmp_image,
                self.tmp_rect.x,
                self.tmp_rect.y,
                self.tmp_rect.width,
                self.tmp_rect.height,
            );
        }
    }

    pub fn draw_image_at(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        image: &TextureRegion,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        let color = self.color;
        let angle = self.angle;
        self.draw_image_at_with_color(
            sprite,
            &DrawImageAtParams {
                image,
                x,
                y,
                width,
                height,
                color: &color,
                angle,
            },
        );
    }

    pub fn draw_image_at_with_color(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        params: &DrawImageAtParams<'_>,
    ) {
        if params.color.a == 0.0 {
            return;
        }
        self.tmp_rect
            .set_xywh(params.x, params.y, params.width, params.height);
        self.stretch
            .stretch_rect(&mut self.tmp_rect, &mut self.tmp_image, params.image);
        sprite.set_color(params.color);
        sprite.blend = self.dstblend;
        sprite.obj_type =
            if self.dstfilter != 0 && self.image_type == SkinObjectRenderer::TYPE_NORMAL {
                if self.tmp_rect.width == self.tmp_image.region_width as f32
                    && self.tmp_rect.height == self.tmp_image.region_height as f32
                {
                    SkinObjectRenderer::TYPE_NORMAL
                } else {
                    SkinObjectRenderer::TYPE_BILINEAR
                }
            } else {
                self.image_type
            };

        if params.angle != 0 {
            sprite.draw_rotated(DrawRotatedParams {
                image: &self.tmp_image,
                x: self.tmp_rect.x,
                y: self.tmp_rect.y,
                w: self.tmp_rect.width,
                h: self.tmp_rect.height,
                cx: self.centerx,
                cy: self.centery,
                angle: params.angle,
            });
        } else {
            sprite.draw(
                &self.tmp_image,
                self.tmp_rect.x,
                self.tmp_rect.y,
                self.tmp_rect.width,
                self.tmp_rect.height,
            );
        }
    }
}
