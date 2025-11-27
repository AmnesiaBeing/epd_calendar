// src/app_core/display_manager.rs
use embassy_time::Instant;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::primitives::Rectangle;
use log::{debug, info, warn};
use std::collections::HashMap;

use crate::common::error::{AppError, Result};

/// 刷新模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RefreshMode {
    /// 全局刷新 - 刷新整个屏幕
    Global,
    /// 局部刷新 - 只刷新变化的部分
    Partial,
}

/// 刷新计划
#[derive(Debug, Clone)]
pub enum RefreshPlan {
    /// 无需刷新
    NoUpdate,
    /// 全局刷新
    Global,
    /// 局部刷新指定区域
    Partial(Rectangle),
}

/// 显示区域信息
#[derive(Debug, Clone)]
pub struct DisplayRegion {
    /// 区域标识符
    pub id: String,
    /// 区域边界
    pub bounds: Rectangle,
    /// 最后更新时间
    pub last_update: Option<Instant>,
    /// 局部刷新次数计数
    pub partial_refresh_count: u32,
    /// 区域是否脏（需要刷新）
    pub is_dirty: bool,
}

impl DisplayRegion {
    pub fn new(id: &str, bounds: Rectangle) -> Self {
        Self {
            id: id.to_string(),
            bounds,
            last_update: None,
            partial_refresh_count: 0,
            is_dirty: false,
        }
    }
}

/// 显示管理器 - 负责协调显示刷新策略
pub struct DisplayManager {
    /// 已注册的显示区域
    regions: HashMap<String, DisplayRegion>,
    /// 当前刷新模式
    refresh_mode: RefreshMode,
    /// 最大局部刷新次数，超过后强制全局刷新
    max_partial_refreshes: u32,
    /// 全局刷新计数器
    global_refresh_counter: u32,
    /// 脏区域列表
    dirty_regions: Vec<String>,
}

impl DisplayManager {
    /// 创建新的显示管理器
    pub fn new(max_partial_refreshes: u32) -> Self {
        Self {
            regions: HashMap::new(),
            refresh_mode: RefreshMode::Global, // 启动时默认全局刷新
            max_partial_refreshes,
            global_refresh_counter: 0,
            dirty_regions: Vec::new(),
        }
    }

    /// 注册显示区域
    pub fn register_region(&mut self, id: &str, bounds: Rectangle) {
        let region = DisplayRegion::new(id, bounds);
        self.regions.insert(id.to_string(), region);
        debug!("Registered display region: {} at {:?}", id, bounds);
    }

    /// 标记区域为脏（需要刷新）
    pub fn mark_dirty(&mut self, region_id: &str) -> Result<()> {
        if let Some(region) = self.regions.get_mut(region_id) {
            if !region.is_dirty {
                region.is_dirty = true;
                region.last_update = Some(Instant::now());
                self.dirty_regions.push(region_id.to_string());
                debug!("Marked region as dirty: {}", region_id);
            }
            Ok(())
        } else {
            warn!("Attempted to mark unknown region as dirty: {}", region_id);
            Err(AppError::ConfigError("Unknown display region"))
        }
    }

    /// 强制全局刷新
    pub fn force_global_refresh(&mut self) {
        self.refresh_mode = RefreshMode::Global;
        debug!("Forced global refresh");
    }

    /// 设置刷新模式
    pub fn set_refresh_mode(&mut self, mode: RefreshMode) {
        self.refresh_mode = mode;
        debug!("Set refresh mode to: {:?}", mode);
    }

    /// 重置所有区域的刷新计数器
    pub fn reset_refresh_counters(&mut self) {
        for region in self.regions.values_mut() {
            region.partial_refresh_count = 0;
        }
        debug!("Reset all refresh counters");
    }

    /// 获取刷新计划
    pub fn get_refresh_plan(&mut self) -> RefreshPlan {
        match self.refresh_mode {
            RefreshMode::Global => self.perform_global_refresh(),
            RefreshMode::Partial => self.perform_partial_refresh(),
        }
    }

    /// 执行全局刷新
    fn perform_global_refresh(&mut self) -> RefreshPlan {
        info!("Planning global refresh");

        // 重置所有区域状态
        for region in self.regions.values_mut() {
            region.is_dirty = false;
            region.partial_refresh_count = 0;
        }
        self.dirty_regions.clear();

        // 切换到局部刷新模式，为下一次刷新做准备
        self.refresh_mode = RefreshMode::Partial;
        self.global_refresh_counter += 1;

        RefreshPlan::Global
    }

    /// 执行局部刷新
    fn perform_partial_refresh(&mut self) -> RefreshPlan {
        // 如果没有脏区域，无需刷新
        if self.dirty_regions.is_empty() {
            return RefreshPlan::NoUpdate;
        }

        // 检查是否需要强制全局刷新（某个区域局部刷新次数过多）
        let needs_global_refresh = self
            .regions
            .values()
            .any(|region| region.partial_refresh_count >= self.max_partial_refreshes);

        if needs_global_refresh {
            info!("Partial refresh limit reached, forcing global refresh");
            return self.perform_global_refresh();
        }

        // 计算需要刷新的区域
        let refresh_area = self.calculate_refresh_area();

        // 更新区域状态
        for region_id in &self.dirty_regions {
            if let Some(region) = self.regions.get_mut(region_id) {
                region.is_dirty = false;
                region.partial_refresh_count += 1;
            }
        }
        self.dirty_regions.clear();

        debug!("Planning partial refresh for area: {:?}", refresh_area);
        RefreshPlan::Partial(refresh_area)
    }

    /// 计算需要刷新的区域（合并所有脏区域）
    fn calculate_refresh_area(&self) -> Rectangle {
        if self.dirty_regions.is_empty() {
            return Rectangle::new(Point::new(0, 0), Size::new(0, 0));
        }

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for region_id in &self.dirty_regions {
            if let Some(region) = self.regions.get(region_id) {
                let bounds = region.bounds;
                min_x = min_x.min(bounds.top_left.x);
                min_y = min_y.min(bounds.top_left.y);
                max_x = max_x.max(bounds.top_left.x + bounds.size.width as i32);
                max_y = max_y.max(bounds.top_left.y + bounds.size.height as i32);
            }
        }

        // 确保坐标有效
        if min_x > max_x || min_y > max_y {
            return Rectangle::new(Point::new(0, 0), Size::new(0, 0));
        }

        Rectangle::new(
            Point::new(min_x, min_y),
            Size::new((max_x - min_x) as u32, (max_y - min_y) as u32),
        )
    }

    /// 获取区域信息
    pub fn get_region(&self, region_id: &str) -> Option<&DisplayRegion> {
        self.regions.get(region_id)
    }

    /// 获取所有区域
    pub fn get_regions(&self) -> &HashMap<String, DisplayRegion> {
        &self.regions
    }

    /// 获取当前刷新模式
    pub fn get_refresh_mode(&self) -> RefreshMode {
        self.refresh_mode
    }

    /// 获取全局刷新计数
    pub fn get_global_refresh_count(&self) -> u32 {
        self.global_refresh_counter
    }

    /// 检查是否有脏区域
    pub fn has_dirty_regions(&self) -> bool {
        !self.dirty_regions.is_empty()
    }

    /// 获取脏区域数量
    pub fn dirty_region_count(&self) -> usize {
        self.dirty_regions.len()
    }

    /// 清除所有脏区域标记
    pub fn clear_dirty_regions(&mut self) {
        for region in self.regions.values_mut() {
            region.is_dirty = false;
        }
        self.dirty_regions.clear();
        debug!("Cleared all dirty regions");
    }
}
