impl TryFrom<&str> for FontSize {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "Small" => Ok(Self::Small),
            "Medium" => Ok(Self::Medium),
            "Large" => Ok(Self::Large),
            _ => Err(format!("无效的字体尺寸: {}", s)),
        }
    }
}
