// ==================== 字体尺寸枚举 ====================
/// 字体尺寸选项
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontSize {
    /// Small字体 (16px)
    Small,
    /// Medium字体 (24px)
    Medium,
    /// Large字体 (40px)
    Large,
}
