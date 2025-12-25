//! 布局规则模块
//! 导入由layout_processor生成的布局规则，供渲染引擎使用

#![allow(dead_code)]

// 导入编译期生成的布局规则
include!(concat!(env!("OUT_DIR"), "/generated_layout.rs"));

/// 获取全局布局规则
pub fn get_global_layout_rules() -> &'static LayoutRules {
    &LAYOUT_RULES
}

/// 根据ID获取布局元素
pub fn get_layout_element(id: &str) -> Option<&'static LayoutElement> {
    LAYOUT_RULES.get_element(id)
}
