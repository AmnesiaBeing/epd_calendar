//! 编译期+运行时共用的极简校验函数
//! 仅包含运行时必须的基础校验（复杂校验放编译期）

use super::*;

/// 运行时校验坐标是否在屏幕内（绝对定位）
pub fn validate_absolute_coord(coord: &[u16], is_2d: bool) -> Result<(), String> {
    let (x, y) = if is_2d {
        (coord[0], coord[1])
    } else {
        (coord[0], coord[1])
    };

    if x > SCREEN_WIDTH || y > SCREEN_HEIGHT {
        return Err(format!(
            "绝对坐标越界: x={} (>{}), y={} (>{})",
            x, SCREEN_WIDTH, y, SCREEN_HEIGHT
        ));
    }

    // 非2D坐标额外校验宽高
    if !is_2d && coord.len() == 4 {
        let width = coord[2];
        let height = coord[3];
        if x + width > SCREEN_WIDTH || y + height > SCREEN_HEIGHT {
            return Err(format!(
                "绝对尺寸越界: x+width={} (>{}), y+height={} (>{})",
                x + width,
                SCREEN_WIDTH,
                y + height,
                SCREEN_HEIGHT
            ));
        }
    }

    Ok(())
}

/// 编译期+运行时通用的权重校验
pub fn validate_weight(weight: &f32) -> Result<(), String> {
    if *weight < MIN_WEIGHT || *weight > MAX_WEIGHT {
        return Err(format!(
            "权重超限: {} (需{}~{})",
            weight, MIN_WEIGHT, MAX_WEIGHT
        ));
    }
    Ok(())
}

/// 编译期+运行时通用的厚度校验
pub fn validate_thickness(thickness: &u16) -> Result<(), String> {
    if *thickness < MIN_THICKNESS || *thickness > MAX_THICKNESS {
        return Err(format!(
            "厚度超限: {} (需{}~{})",
            thickness, MIN_THICKNESS, MAX_THICKNESS
        ));
    }
    Ok(())
}
