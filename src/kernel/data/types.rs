// src/core/data/types.rs
//! 数据类型定义模块
//! 定义数据源系统中使用的核心数据类型

use alloc::{boxed::Box, collections::btree_map::BTreeMap};
use async_trait::async_trait;
use core::fmt::Debug;
use embassy_time::Duration;
use serde::{Deserialize, Serialize};

// 创建常用的类型别名
pub type HeaplessString<const N: usize> = heapless::String<N>;
pub type HeaplessVec<T, const N: usize> = heapless::Vec<T, N>;

pub const KEY_LENGTH: usize = 32;
pub const VALUE_LENGTH: usize = 32;
pub type CacheKey = HeaplessString<KEY_LENGTH>;
pub type CacheStringValue = HeaplessString<VALUE_LENGTH>;
pub type CacheKeyValueMap = BTreeMap<CacheKey, DynamicValue>;

use crate::{
    common::{
        GlobalMutex, GlobalRwLockWriteGuard,
        error::{AppError, Result},
    },
    kernel::system::api::DefaultSystemApi,
};

#[inline]
pub fn alloc_string_to_heapless<const N: usize>(s: &str) -> Result<HeaplessString<N>> {
    Ok(unsafe {
        HeaplessString::from_utf8_unchecked(
            HeaplessVec::from_slice(s.as_bytes()).map_err(|_| AppError::ConvertError)?,
        )
    })
}

/// 数据源接口定义
#[async_trait(?Send)]
pub trait DataSource {
    /// 获取数据源名称
    fn name(&self) -> &'static str;

    /// 刷新数据并直接写入缓存（核心修改：替代原 refresh + get_all_fields）
    /// 参数说明：
    /// - system_api: 系统API
    /// 核心变更：刷新数据并直接写入全局缓存（替代原 refresh 方法）
    async fn refresh_with_cache(
        &mut self,
        system_api: &'static GlobalMutex<DefaultSystemApi>,
        cache_guard: &mut GlobalRwLockWriteGuard<CacheKeyValueMap>,
    ) -> Result<()>;

    /// 获取刷新间隔（ticks）
    fn refresh_interval(&self) -> Duration;
}

/// 动态值枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DynamicValue {
    /// 布尔值
    Boolean(bool),
    /// 整数
    Integer(i32),
    /// 浮点数
    Float(f32),
    /// 字符串
    String(CacheStringValue),
}
