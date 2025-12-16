//! 布局节点定义
//! 定义所有布局节点的数据结构，与 builder/modules/layout_processor.rs 中的结构保持一致
#![allow(unused)]

pub use crate::assets::generated_fonts::FontSize;

type NodeIdStr = &'static str;
type IconId = &'static str;
type Content = &'static str;
type Condition = &'static str;
type LayoutNodeEntryVec = &'static [LayoutPoolEntry];
type ChildLayoutVec = &'static [NodeId];

include!("../../../../shared/layout_types.rs");
