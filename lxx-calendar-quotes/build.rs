//! 格言数据处理模块

#![allow(unused)]

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Hitokoto {
    pub hitokoto: String,
    pub from: String,
    pub from_who: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HitokotoCategory {
    pub id: u32,
    #[allow(dead_code)]
    pub name: String,
    pub key: String,
}

/// 构建格言数据
pub fn main() -> Result<()> {
    let categories = parse_categories()?;
    let hitokotos = parse_all_json_files(&categories)?;

    generate_hitokoto_data(&hitokotos)?;

    Ok(())
}

pub fn parse_categories() -> Result<Vec<HitokotoCategory>> {
    let path = PathBuf::from("sentences").join("categories.json");
    let content = fs::read_to_string(&path)
        .with_context(|| format!("读取分类文件失败: {}", path.display()))?;
    serde_json::from_str(&content).context("解析categories.json失败")
}

pub fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

pub fn parse_all_json_files(categories: &[HitokotoCategory]) -> Result<Vec<(u32, Vec<Hitokoto>)>> {
    let mut result = Vec::new();
    let mut total_hitokotos = 0;
    let mut valid_hitokotos = 0;
    let mut ignored_hitokotos = 0;

    for (index, category) in categories.iter().enumerate() {
        let mut path = PathBuf::from("sentences").join("sentences");
        path.push(&category.key);
        path.set_extension("json");

        let content = fs::read_to_string(&path)
            .with_context(|| format!("读取JSON文件失败: {}", path.display()))?;

        let hitokotos: Vec<Hitokoto> = serde_json::from_str(&content)
            .with_context(|| format!("解析JSON失败: {}", path.display()))?;

        result.push((category.id, hitokotos));
    }

    Ok(result)
}

fn generate_hitokoto_data(hitokotos: &[(u32, Vec<Hitokoto>)]) -> Result<()> {
    let output_path = PathBuf::from(std::env::var("OUT_DIR")?).join("generated_hitokoto_data.rs");

    let mut from_strings = BTreeSet::new();
    let mut from_who_strings = BTreeSet::new();
    let mut all_hitokotos = Vec::new();

    from_who_strings.insert("佚名".to_string());

    for (category_id, hitokoto_list) in hitokotos {
        for hitokoto in hitokoto_list {
            from_strings.insert(hitokoto.from.clone());
            if let Some(from_who) = &hitokoto.from_who {
                from_who_strings.insert(from_who.clone());
            }
            all_hitokotos.push((*category_id, hitokoto));
        }
    }

    let from_vec: Vec<String> = from_strings.into_iter().collect();
    let from_who_vec: Vec<String> = from_who_strings.into_iter().collect();

    let from_index_map: HashMap<&str, usize> = from_vec
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();

    let from_who_index_map: HashMap<&str, usize> = from_who_vec
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();

    let yiming_index = from_who_index_map["佚名"];

    let mut content = String::new();
    content.push_str("// 自动生成的格言数据文件\n");
    content.push_str("// 不要手动修改此文件\n\n");

    // 生成 FROM_STRINGS 数组
    content.push_str("pub const FROM_STRINGS: &[&str] = &[\n");
    for from_str in &from_vec {
        content.push_str(&format!("    \"{}\",\n", escape_string(from_str)));
    }
    content.push_str("];\n\n");

    // 生成 FROM_WHO_STRINGS 数组
    content.push_str("pub const FROM_WHO_STRINGS: &[&str] = &[\n");
    for from_who_str in &from_who_vec {
        content.push_str(&format!("    \"{}\",\n", escape_string(from_who_str)));
    }
    content.push_str("];\n\n");

    // 生成 Hitokoto 结构体和数据数组
    content.push_str("#[derive(Clone, Copy)]\n");
    content.push_str("#[allow(dead_code)]\n");
    content.push_str("#[repr(C, packed)]\n");
    content.push_str("pub struct Hitokoto {\n");
    content.push_str("    pub hitokoto: &'static str,\n");
    content.push_str("    pub from: u16,\n");
    content.push_str("    pub from_who: u16,\n");
    content.push_str("}\n\n");
    content.push_str("pub const HITOKOTOS: &[Hitokoto] = &[\n");

    for (_, hitokoto) in &all_hitokotos {
        let from_index = from_index_map[hitokoto.from.as_str()];
        let from_who_index = if let Some(from_who) = &hitokoto.from_who {
            from_who_index_map[from_who.as_str()]
        } else {
            yiming_index
        };

        content.push_str("    Hitokoto {\n");
        content.push_str(&format!(
            "        hitokoto: \"{}\",\n",
            escape_string(&hitokoto.hitokoto)
        ));
        content.push_str(&format!("        from: {},\n", from_index));
        content.push_str(&format!("        from_who: {},\n", from_who_index));
        content.push_str("    },\n");
    }
    content.push_str("];\n");

    fs::write(&output_path, content)?;

    Ok(())
}
