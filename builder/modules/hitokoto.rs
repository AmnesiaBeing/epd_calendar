//! 格言数据处理模块

use crate::builder::config::BuildConfig;
use crate::builder::utils::{self, progress::ProgressTracker};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use std::fs;

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
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    progress.update_progress(0, 3, "解析分类");
    let categories = parse_categories(config)?;

    progress.update_progress(1, 3, "解析格言文件");
    let hitokotos = parse_all_json_files(config, &categories)?;

    progress.update_progress(2, 3, "生成数据文件");
    generate_hitokoto_data(config, &hitokotos)?;

    println!(
        "cargo:warning=  格言数据处理完成，共处理 {} 个分类",
        categories.len()
    );

    Ok(())
}

pub fn parse_categories(config: &BuildConfig) -> Result<Vec<HitokotoCategory>> {
    let content = fs::read_to_string(&config.categories_path)
        .with_context(|| format!("读取分类文件失败: {}", config.categories_path.display()))?;
    serde_json::from_str(&content).context("解析categories.json失败")
}

pub fn parse_all_json_files(
    config: &BuildConfig,
    categories: &[HitokotoCategory],
) -> Result<Vec<(u32, Vec<Hitokoto>)>> {
    let mut result = Vec::new();

    for (index, category) in categories.iter().enumerate() {
        let mut path = config.sentences_dir.join(&category.key);
        path.set_extension("json");

        let content = fs::read_to_string(&path)
            .with_context(|| format!("读取JSON文件失败: {}", path.display()))?;

        let hitokotos: Vec<Hitokoto> = serde_json::from_str(&content)
            .with_context(|| format!("解析JSON失败: {}", path.display()))?;

        result.push((category.id, hitokotos));

        // 更新进度
        println!(
            "cargo:warning=  已处理分类: {}/{}",
            index + 1,
            categories.len()
        );
    }

    Ok(result)
}

fn generate_hitokoto_data(config: &BuildConfig, hitokotos: &[(u32, Vec<Hitokoto>)]) -> Result<()> {
    let output_path = config.output_dir.join("generated_hitokoto_data.rs");

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
        content.push_str(&format!(
            "    \"{}\",\n",
            utils::string_utils::escape_string(from_str)
        ));
    }
    content.push_str("];\n\n");

    // 生成 FROM_WHO_STRINGS 数组
    content.push_str("pub const FROM_WHO_STRINGS: &[&str] = &[\n");
    for from_who_str in &from_who_vec {
        content.push_str(&format!(
            "    \"{}\",\n",
            utils::string_utils::escape_string(from_who_str)
        ));
    }
    content.push_str("];\n\n");

    // 生成 Hitokoto 结构体和数据数组
    content.push_str("#[derive(Clone, Copy)]\n");
    content.push_str("#[allow(dead_code)]\n");
    content.push_str("pub struct Hitokoto {\n");
    content.push_str("    pub hitokoto: &'static str,\n");
    content.push_str("    pub from: usize,\n");
    content.push_str("    pub from_who: usize,\n");
    content.push_str("    pub category: u32,\n");
    content.push_str("}\n\n");
    content.push_str("pub const HITOKOTOS: &[Hitokoto] = &[\n");

    for (category_id, hitokoto) in &all_hitokotos {
        let from_index = from_index_map[hitokoto.from.as_str()];
        let from_who_index = if let Some(from_who) = &hitokoto.from_who {
            from_who_index_map[from_who.as_str()]
        } else {
            yiming_index
        };

        content.push_str("    Hitokoto {\n");
        content.push_str(&format!(
            "        hitokoto: \"{}\",\n",
            utils::string_utils::escape_string(&hitokoto.hitokoto)
        ));
        content.push_str(&format!("        from: {},\n", from_index));
        content.push_str(&format!("        from_who: {},\n", from_who_index));
        content.push_str(&format!("        category: {},\n", category_id));
        content.push_str("    },\n");
    }
    content.push_str("];\n");

    utils::file_utils::write_string_file(&output_path, &content)?;

    println!(
        "cargo:warning=  生成格言数据: 来源{}个, 作者{}个, 格言{}条",
        from_vec.len(),
        from_who_vec.len(),
        all_hitokotos.len()
    );

    Ok(())
}
