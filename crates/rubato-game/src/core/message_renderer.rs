use rubato_render::font::BitmapFont;
use rubato_render::sprite_batch::SpriteBatch;

/// Color - RGBA color (LibGDX equivalent)
#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const CYAN: Color = Color {
        r: 0.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

/// Message - a message rendered by MessageRenderer
pub struct Message {
    time: i64,
    text: String,
    color: Color,
    message_type: i32,
    font: Option<BitmapFont>,
}

impl Message {
    pub fn new(text: &str, time: i64, color: Color, message_type: i32) -> Self {
        let now_millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_millis() as i64;
        Self {
            time: time + now_millis,
            text: text.to_string(),
            color,
            message_type,
            font: None,
        }
    }

    /// Initialize font for this message.
    ///
    /// Translated from: Message.init(FreeTypeFontGenerator)
    /// In Java, this generates a BitmapFont from the FreeTypeFontGenerator with
    /// size=24, color=self.color.
    pub fn init(&mut self, fontpath: &str) {
        let mut font = BitmapFont::from_file(fontpath, 24.0);
        font.set_color(&rubato_render::color::Color::new(
            self.color.r,
            self.color.g,
            self.color.b,
            self.color.a,
        ));
        self.font = Some(font);
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
    }

    pub fn stop(&mut self) {
        self.time = -1;
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn color(&self) -> &Color {
        &self.color
    }

    pub fn message_type(&self) -> i32 {
        self.message_type
    }

    pub fn is_expired(&self) -> bool {
        let now_millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_millis() as i64;
        self.time < now_millis
    }

    /// Draw this message's text at (x, y) with alpha pulsing animation.
    ///
    /// Translated from Java: Message.draw(MainState, SpriteBatch, int, int)
    /// Alpha animation: sinDeg((millis % 1440) / 4.0) * 0.3 + 0.7
    pub fn draw(&mut self, sprite: &mut SpriteBatch, x: i32, y: i32) {
        let Some(font) = self.font.as_mut() else {
            return;
        };

        // Alpha pulsing: Java MathUtils.sinDeg((System.currentTimeMillis() % 1440) / 4.0f) * 0.3f + 0.7f
        let now_millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_millis() as u64;
        let cycle = (now_millis % 1440) as f32;
        let alpha = (cycle / 4.0).to_radians().sin() * 0.3 + 0.7;

        font.set_color(&rubato_render::color::Color::new(
            self.color.r,
            self.color.g,
            self.color.b,
            alpha,
        ));
        font.draw(sprite, &self.text, x as f32, y as f32);
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut font) = self.font {
            font.dispose();
        }
        self.font = None;
    }
}

/// MessageRenderer - renders messages on screen
pub struct MessageRenderer {
    messages: Vec<Message>,
    fontpath: String,
}

impl MessageRenderer {
    pub fn new(fontpath: &str) -> Self {
        Self {
            messages: Vec::new(),
            fontpath: fontpath.to_string(),
        }
    }

    /// Render messages, removing expired ones.
    ///
    /// Translated from Java: MessageRenderer.render(MainState, SpriteBatch, int, int)
    pub fn render(&mut self, sprite: &mut SpriteBatch, x: i32, y: i32) {
        let mut dy = 0;
        let mut i = self.messages.len();
        while i > 0 {
            i -= 1;
            if self.messages[i].is_expired() {
                self.messages[i].dispose();
                self.messages.remove(i);
            } else {
                self.messages[i].draw(sprite, x, y - dy);
                dy += 24;
            }
        }
    }

    pub fn add_message(&mut self, text: &str, color: Color, message_type: i32) -> &Message {
        self.add_message_with_time(text, 24 * 60 * 60 * 1000, color, message_type)
    }

    pub fn add_message_with_time(
        &mut self,
        text: &str,
        time: i64,
        color: Color,
        message_type: i32,
    ) -> &Message {
        let mut message = Message::new(text, time, color, message_type);
        message.init(&self.fontpath);
        self.messages.push(message);
        self.messages.last().expect("non-empty")
    }

    pub fn dispose(&mut self) {
        for msg in &mut self.messages {
            msg.dispose();
        }
        self.messages.clear();
    }
}
