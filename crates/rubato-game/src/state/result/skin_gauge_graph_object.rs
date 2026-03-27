// SkinGaugeGraphObject.java -> skin_gauge_graph_object.rs
// Mechanical line-by-line translation.

use super::{
    Color, Pixmap, PixmapFormat, PlayerResource, SkinObjectData, SkinObjectRenderer, Texture,
    TextureRegion,
};

/// Color strings for constructing a gauge graph from hex color values.
pub struct GaugeGraphStringColors<'a> {
    pub assist_clear_bg_color: &'a str,
    pub assist_and_easy_fail_bg_color: &'a str,
    pub groove_fail_bg_color: &'a str,
    pub groove_clear_and_hard_bg_color: &'a str,
    pub ex_hard_bg_color: &'a str,
    pub hazard_bg_color: &'a str,
    pub assist_clear_line_color: &'a str,
    pub assist_and_easy_fail_line_color: &'a str,
    pub groove_fail_line_color: &'a str,
    pub groove_clear_and_hard_line_color: &'a str,
    pub ex_hard_line_color: &'a str,
    pub hazard_line_color: &'a str,
    pub borderline_color: &'a str,
    pub border_color: &'a str,
}

/// Gauge graph rendering object for result screen
pub struct SkinGaugeGraphObject {
    /// Background texture
    backtex: Option<TextureRegion>,
    /// Graph texture
    shapetex: Option<TextureRegion>,
    /// Delay time for gauge graph rendering (ms)
    pub delay: i32,
    /// Graph line width
    pub line_width: i32,

    /// Background color below border
    graphcolor: [Color; 6],
    /// Graph line color below border
    graphline: [Color; 6],
    /// Background color above border
    borderline: [Color; 6],
    /// Graph line color above border
    bordercolor: [Color; 6],

    typetable: [i32; 10],

    current_type: i32,
    color: usize,
    gaugehistory: Vec<f32>,
    section: Vec<i32>,
    gg: Option<GaugeRef>,

    render: f32,
    redraw: bool,

    /// SkinObject base data
    pub object_data: SkinObjectData,
}

/// Reference to a Gauge (non-owning, for rendering)
pub struct GaugeRef {
    pub border: f32,
    pub max: f32,
}

impl SkinGaugeGraphObject {
    pub fn new() -> Self {
        Self::new_with_colors(&[
            &[
                Color::value_of("ff0000"),
                Color::value_of("440000"),
                Color::value_of("ff00ff"),
                Color::value_of("440044"),
            ],
            &[
                Color::value_of("ff0000"),
                Color::value_of("440000"),
                Color::value_of("00ffff"),
                Color::value_of("004444"),
            ],
            &[
                Color::value_of("ff0000"),
                Color::value_of("440000"),
                Color::value_of("00ff00"),
                Color::value_of("004400"),
            ],
            &[Color::value_of("ff0000"), Color::value_of("440000")],
            &[Color::value_of("ffff00"), Color::value_of("444400")],
            &[Color::value_of("cccccc"), Color::value_of("444444")],
        ])
    }

    pub fn new_with_colors(colors: &[&[Color]]) -> Self {
        let black = || Color::value_of("000000");
        let mut graphcolor: [Color; 6] = Default::default();
        let mut graphline: [Color; 6] = Default::default();
        let mut borderline: [Color; 6] = Default::default();
        let mut bordercolor: [Color; 6] = Default::default();

        for i in 0..6 {
            if colors.len() > i {
                borderline[i] = if !colors[i].is_empty() {
                    colors[i][0]
                } else {
                    black()
                };
                bordercolor[i] = if colors[i].len() > 1 {
                    colors[i][1]
                } else {
                    black()
                };
                graphline[i] = if colors[i].len() > 2 {
                    colors[i][2]
                } else {
                    black()
                };
                graphcolor[i] = if colors[i].len() > 3 {
                    colors[i][3]
                } else {
                    black()
                };
            } else {
                graphline[i] = black();
                graphcolor[i] = black();
                borderline[i] = black();
                bordercolor[i] = black();
            }
        }

        Self {
            backtex: None,
            shapetex: None,
            delay: 1500,
            line_width: 2,
            graphcolor,
            graphline,
            borderline,
            bordercolor,
            typetable: [0, 1, 2, 3, 4, 5, 3, 4, 5, 3],
            current_type: -1,
            color: 0,
            gaugehistory: Vec::new(),
            section: Vec::new(),
            gg: None,
            render: 0.0,
            redraw: false,
            object_data: SkinObjectData::new(),
        }
    }

    pub fn new_with_string_colors(colors: &GaugeGraphStringColors<'_>) -> Self {
        let mut graphcolor: [Color; 6] = Default::default();
        let mut graphline: [Color; 6] = Default::default();
        let mut borderline: [Color; 6] = Default::default();
        let mut bordercolor: [Color; 6] = Default::default();

        graphcolor[0] = Color::value_of(colors.assist_clear_bg_color);
        graphcolor[1] = Color::value_of(colors.assist_and_easy_fail_bg_color);
        graphcolor[2] = Color::value_of(colors.groove_fail_bg_color);
        bordercolor[3] = Color::value_of(colors.groove_clear_and_hard_bg_color);
        bordercolor[4] = Color::value_of(colors.ex_hard_bg_color);
        bordercolor[5] = Color::value_of(colors.hazard_bg_color);
        graphline[0] = Color::value_of(colors.assist_clear_line_color);
        graphline[1] = Color::value_of(colors.assist_and_easy_fail_line_color);
        graphline[2] = Color::value_of(colors.groove_fail_line_color);
        borderline[3] = Color::value_of(colors.groove_clear_and_hard_line_color);
        borderline[4] = Color::value_of(colors.ex_hard_line_color);
        borderline[5] = Color::value_of(colors.hazard_line_color);

        for i in 0..3 {
            borderline[i] = Color::value_of(colors.borderline_color);
            bordercolor[i] = Color::value_of(colors.border_color);
        }
        graphline[3..6].clone_from_slice(&borderline[3..6]);
        graphcolor[3..6].clone_from_slice(&bordercolor[3..6]);

        Self {
            backtex: None,
            shapetex: None,
            delay: 1500,
            line_width: 2,
            graphcolor,
            graphline,
            borderline,
            bordercolor,
            typetable: [0, 1, 2, 3, 4, 5, 3, 4, 5, 3],
            current_type: -1,
            color: 0,
            gaugehistory: Vec::new(),
            section: Vec::new(),
            gg: None,
            render: 0.0,
            redraw: false,
            object_data: SkinObjectData::new(),
        }
    }

    pub fn prepare(
        &mut self,
        time: i64,
        _gauge_type: i32,
        result_gauge_type: i32,
        resource: &PlayerResource,
        is_course_result: bool,
    ) {
        self.render = if self.delay <= 0 || time >= self.delay as i64 {
            1.0
        } else {
            time as f32 / self.delay as f32
        };

        // In Java: type = resource.getGrooveGauge().getType();
        // if (state instanceof AbstractResult) type = ((AbstractResult) state).gaugeType;
        let current_type = result_gauge_type;

        if current_type < 0 {
            return;
        }

        if self.current_type != current_type {
            self.redraw = true;
            self.current_type = current_type;
            self.gaugehistory = resource
                .gauge()
                .and_then(|gd| gd.get(self.current_type as usize))
                .cloned()
                .unwrap_or_default();
            self.section = Vec::new();
            if is_course_result {
                self.gaugehistory = Vec::new();
                for l in resource.course_gauge() {
                    if let Some(gauge_data) = l.get(self.current_type as usize) {
                        self.gaugehistory.extend_from_slice(gauge_data);
                        let prev = self.section.last().copied().unwrap_or(0);
                        self.section.push(prev + gauge_data.len() as i32);
                    }
                }
            }
            if let Some(groove_gauge) = resource.groove_gauge() {
                let gauge = groove_gauge.gauge_by_type(self.current_type);
                let prop = gauge.property();
                self.gg = Some(GaugeRef {
                    border: prop.border,
                    max: prop.max,
                });
            }
        }
        // super.prepare(time, state) would go here
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        // Copy region dimensions to avoid borrow conflicts
        let region_x = self.object_data.region.x;
        let region_y = self.object_data.region.y;
        let region_width = self.object_data.region.width;
        let region_height = self.object_data.region.height;

        {
            let needs_dispose = if let Some(ref shapetex) = self.shapetex {
                if !self.redraw {
                    if let Some(tex) = shapetex.texture.as_ref() {
                        tex.width != region_width as i32 || tex.height != region_height as i32
                    } else {
                        false
                    }
                } else {
                    true
                }
            } else {
                false
            };
            if needs_dispose {
                self.dispose_textures();
            }
        }

        if self.shapetex.is_none() {
            self.redraw = false;
            let width = region_width as i32;
            let height = region_height as i32;

            // Create background pixmap
            let mut shape = Pixmap::new(width, height, PixmapFormat::RGBA8888);
            let type_idx = (self.current_type as usize).min(self.typetable.len().saturating_sub(1));
            self.color = self.typetable[type_idx] as usize;
            shape.set_color(&self.graphcolor[self.color]);
            shape.fill();

            if let Some(ref gg) = self.gg {
                let border = gg.border;
                let max = gg.max;
                if max > 0.0 {
                    shape.set_color(&self.bordercolor[self.color]);
                    shape.fill_rectangle(
                        0,
                        (height as f32 * border / max) as i32,
                        width,
                        (height as f32 * (max - border) / max) as i32,
                    );

                    self.backtex = Some(TextureRegion::from_texture(Texture::from_pixmap(&shape)));
                    shape.dispose();

                    // Create graph pixmap
                    let mut shape = Pixmap::new(width, height, PixmapFormat::RGBA8888);
                    let mut f1: Option<f32> = None;
                    let mut last_gauge: f32 = -1.0;
                    let mut last_x: i32 = -1;
                    let mut last_y: i32 = -1;
                    let line_width = self.line_width;

                    let gauge_len = self.gaugehistory.len() as f32;
                    for (i, &f2) in self.gaugehistory.iter().enumerate() {
                        if self.section.contains(&(i as i32)) {
                            shape.set_color(&Color::value_of("ffffff"));
                            shape.draw_line(
                                (width as f32 * (i as f32 - 1.0) / gauge_len) as i32,
                                0,
                                (width as f32 * (i as f32 - 1.0) / gauge_len) as i32,
                                height,
                            );
                        }
                        if let Some(f1_val) = f1 {
                            let x1 = (width as f32 * (i as f32 - 1.0) / gauge_len) as i32;
                            let y1 = ((f1_val / max) * (height - line_width) as f32) as i32;
                            let x2 = (width as f32 * i as f32 / gauge_len) as i32;
                            let y2 = ((f2 / max) * (height - line_width) as f32) as i32;
                            let yb = ((border / max) * (height - line_width) as f32) as i32;
                            last_gauge = f2;
                            last_x = x2;
                            last_y = y2;
                            if f1_val < border {
                                if f2 < border {
                                    shape.set_color(&self.graphline[self.color]);
                                    shape.fill_rectangle(
                                        x1,
                                        y1.min(y2),
                                        line_width,
                                        (y2 - y1).abs() + line_width,
                                    );
                                    shape.fill_rectangle(x1, y2, x2 - x1, line_width);
                                } else {
                                    shape.set_color(&self.graphline[self.color]);
                                    shape.fill_rectangle(x1, y1, line_width, yb - y1);
                                    shape.set_color(&self.borderline[self.color]);
                                    shape.fill_rectangle(x1, yb, line_width, y2 - yb + line_width);
                                    shape.fill_rectangle(x1, y2, x2 - x1, line_width);
                                }
                            } else if f2 >= border {
                                shape.set_color(&self.borderline[self.color]);
                                shape.fill_rectangle(
                                    x1,
                                    y1.min(y2),
                                    line_width,
                                    (y2 - y1).abs() + line_width,
                                );
                                shape.fill_rectangle(x1, y2, x2 - x1, line_width);
                            } else {
                                shape.set_color(&self.borderline[self.color]);
                                shape.fill_rectangle(x1, yb, line_width, y1 - yb + line_width);
                                shape.set_color(&self.graphline[self.color]);
                                shape.fill_rectangle(x1, y2, line_width, yb - y2);
                                shape.fill_rectangle(x1, y2, x2 - x1, line_width);
                            }
                        }
                        f1 = Some(f2);
                    }

                    if last_gauge != -1.0 {
                        if last_gauge < border {
                            shape.set_color(&self.graphline[self.color]);
                        } else {
                            shape.set_color(&self.borderline[self.color]);
                        }
                        shape.fill_rectangle(last_x, last_y, width - last_x, line_width);
                    }

                    self.shapetex = Some(TextureRegion::from_texture(Texture::from_pixmap(&shape)));
                    shape.dispose();
                }
            }
        }

        // Draw background
        if let Some(ref backtex) = self.backtex {
            sprite.draw(
                backtex,
                region_x,
                region_y + region_height,
                region_width,
                -region_height,
            );
        }
        // Draw graph with render progress
        if let Some(ref mut shapetex) = self.shapetex {
            shapetex.set_region_from(
                0,
                0,
                (region_width * self.render) as i32,
                region_height as i32,
            );
            sprite.draw(
                shapetex,
                region_x,
                region_y + region_height,
                region_width * self.render,
                -region_height,
            );
        }
    }

    fn dispose_textures(&mut self) {
        if let Some(ref mut tex) = self.shapetex
            && let Some(t) = tex.texture.as_mut()
        {
            t.dispose();
        }
        self.shapetex = None;
        if let Some(ref mut tex) = self.backtex
            && let Some(t) = tex.texture.as_mut()
        {
            t.dispose();
        }
        self.backtex = None;
    }

    pub fn delay(&self) -> i32 {
        self.delay
    }

    pub fn line_width(&self) -> i32 {
        self.line_width
    }

    pub fn dispose(&mut self) {
        self.dispose_textures();
    }
}

impl Default for SkinGaugeGraphObject {
    fn default() -> Self {
        Self::new()
    }
}
