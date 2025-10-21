//! 格言显示功能

use embedded_graphics::{
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};

/// 格言管理器
/// 从一言网获取
pub struct QuoteManager {
    quotes: Vec<&'static str>,
    current_index: usize,
}

impl QuoteManager {
    /// 创建新的格言管理器
    pub fn new() -> Self {
        Self {
            quotes: vec![
                "知识就是力量。",
                "时间就是金钱。",
                "坚持就是胜利。",
                "机会总是留给有准备的人。",
                "失败乃成功之母。",
                "一分耕耘，一分收获。",
                "三人行，必有我师焉。",
                "己所不欲，勿施于人。",
            ],
            current_index: 0,
        }
    }

    /// 获取当前格言
    pub fn current_quote(&self) -> &str {
        self.quotes[self.current_index]
    }

    /// 切换到下一条格言
    pub fn next_quote(&mut self) {
        self.current_index = (self.current_index + 1) % self.quotes.len();
    }

    /// 随机选择一条格言
    pub fn random_quote(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();
        self.current_index = rng.gen_range(0..self.quotes.len());
    }
}

/// 绘制格言到缓冲区
pub fn draw_quote(
    buffer: &mut super::super::graphics::buffer::FrameBuffer,
    face: &mut Face,
    quote: &str,
) -> Result<(), freetype::Error> {
    draw_string(buffer, face, quote, 100, 380, Color::Red, 26)
}
