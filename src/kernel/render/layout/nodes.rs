//! 布局节点定义
//! 定义所有布局节点的数据结构，与 builder/modules/layout_processor.rs 中的结构保持一致
#![allow(unused)]

pub use crate::assets::generated_fonts::FontSize;

type Id = &'static str;
type IconId = &'static str;
type Content = &'static str;
type Condition = &'static str;
type LayoutNodeVec = &'static [LayoutNode];
type ChildLayoutVec = &'static [ChildLayout];

include!("../../../../shared/layout_types.rs");
