//! 布局节点定义
//! 定义所有布局节点的数据结构，与 builder/modules/layout_processor.rs 中的结构保持一致
#![allow(unused)]

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};

/// 字体大小
pub use crate::assets::generated_fonts::FontSize;

pub use crate::shared::layout_types::*;
