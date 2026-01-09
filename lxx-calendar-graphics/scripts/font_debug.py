#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
字体 bin 文件调试工具
支持功能：
1. 解析 generated_fonts.rs 提取字符表和度量参数
2. 查看字符度量参数
3. 可视化字符位图（文本/图片）
4. 验证 bin 文件完整性
5. 统计字体文件信息
"""

import re
import argparse
import os
import sys
from typing import Dict, List, Optional, Tuple

try:
    from PIL import Image, ImageDraw
except ImportError:
    print("警告：未安装 Pillow 库，图片渲染功能不可用！")
    print("安装命令：pip install pillow")
    Image = None


# -------------------------- 核心解析函数 --------------------------
def parse_generated_fonts_rs(rs_file_path: str) -> Dict:
    """
    解析 generated_fonts.rs 文件，提取字体相关信息
    返回字典结构：
    {
        "chars": [char1, char2, ...],          # 共享字符表
        "missing_chars": [char1, char2, ...],  # 缺失字符表
        "font_sizes": {                        # 字体尺寸映射
            "Small": 16,
            "Medium": 24,
            ...
        },
        "metrics": {                           # 各字体的度量参数
            "Small": [
                {"offset": 0, "width": 8, "height": 16, "bearing_x": 0, "bearing_y": 14, "advance_x": 9},
                ...
            ],
            ...
        }
    }
    """
    if not os.path.exists(rs_file_path):
        raise FileNotFoundError(f"Rust 文件不存在: {rs_file_path}")

    with open(rs_file_path, "r", encoding="utf-8") as f:
        content = f.read()

    result = {"chars": [], "missing_chars": [], "font_sizes": {}, "metrics": {}}

    # 1. 解析共享字符表 CHARS
    chars_match = re.search(r"pub const CHARS: &\[char\] = &\[([\s\S]*?)\];", content)
    if chars_match:
        chars_block = chars_match.group(1)
        # 提取所有 'x' 格式的字符，处理转义
        char_pattern = re.compile(r"'(.*?)'")
        char_matches = char_pattern.findall(chars_block)
        for c in char_matches:
            if c == "\\'":
                result["chars"].append("'")
            elif c == "\\\\":
                result["chars"].append("\\")
            elif c:
                result["chars"].append(c)

    # 2. 解析缺失字符表 MISSING_CHARS
    missing_match = re.search(
        r"pub const MISSING_CHARS: &\[char\] = &\[([\s\S]*?)\];", content
    )
    if missing_match:
        missing_block = missing_match.group(1)
        char_pattern = re.compile(r"'(.*?)'")
        char_matches = char_pattern.findall(missing_block)
        for c in char_matches:
            if c == "\\'":
                result["missing_chars"].append("'")
            elif c == "\\\\":
                result["missing_chars"].append("\\")
            elif c:
                result["missing_chars"].append(c)

    # 3. 解析字体尺寸枚举（FontSize）
    font_size_match = re.search(r"pub enum FontSize \{([\s\S]*?)\}", content)
    if font_size_match:
        font_size_block = font_size_match.group(1)
        # 匹配 /// XXX字体 (XXpx) 注释和对应的枚举值
        size_pattern = re.compile(r"/// (.*?)字体 \((\d+)px\)\s+(\w+),")
        size_matches = size_pattern.findall(font_size_block)
        for name_cn, size, name_en in size_matches:
            result["font_sizes"][name_en] = int(size)

    # 4. 解析各字体的度量参数
    # 匹配 FONT_XXX_METRICS = &[ ... ] 块
    metrics_pattern = re.compile(
        r"pub const FONT_(\w+)_METRICS: &\[GlyphMetrics\] = &\[([\s\S]*?)\];"
    )
    metrics_matches = metrics_pattern.findall(content)
    for font_name_upper, metrics_block in metrics_matches:
        font_name = font_name_upper.lower().capitalize()  # Small/Medium/Large
        if font_name not in result["font_sizes"]:
            continue

        # 解析每个 GlyphMetrics 结构体
        glyph_pattern = re.compile(
            r"GlyphMetrics \{\s+"
            r"offset: (\d+),\s+"
            r"width: (\d+),\s+"
            r"height: (\d+),\s+"
            r"bearing_x: (-?\d+),\s+"
            r"bearing_y: (-?\d+),\s+"
            r"advance_x: (-?\d+),\s+"
            r"\},"
        )
        glyph_matches = glyph_pattern.findall(metrics_block)
        metrics_list = []
        for match in glyph_matches:
            metrics = {
                "offset": int(match[0]),
                "width": int(match[1]),
                "height": int(match[2]),
                "bearing_x": int(match[3]),
                "bearing_y": int(match[4]),
                "advance_x": int(match[5]),
            }
            metrics_list.append(metrics)
        result["metrics"][font_name] = metrics_list

    return result


def read_font_bin(bin_file_path: str) -> bytes:
    """读取字体 bin 文件"""
    if not os.path.exists(bin_file_path):
        raise FileNotFoundError(f"Bin 文件不存在: {bin_file_path}")
    with open(bin_file_path, "rb") as f:
        return f.read()


# -------------------------- 位图渲染函数 --------------------------
def get_glyph_bitmap_data(
    font_data: bytes, metrics: Dict, char_index: int
) -> Tuple[bytes, int, int]:
    """
    获取字符的位图原始数据
    返回：(位图数据, 宽度, 高度)
    """
    char_metrics = metrics[char_index]
    width = char_metrics["width"]
    height = char_metrics["height"]
    offset = char_metrics["offset"]

    # 计算字节数：(宽度 +7) //8 * 高度
    bytes_per_row = (width + 7) // 8
    data_len = bytes_per_row * height

    # 验证偏移和长度
    if offset + data_len > len(font_data):
        raise ValueError(f"字符索引 {char_index} 的位图数据越界！")

    bitmap_data = font_data[offset : offset + data_len]
    return bitmap_data, width, height


def render_bitmap_text(bitmap_data: bytes, width: int, height: int) -> str:
    """
    将位图数据渲染为文本形式
    使用 ■ 表示1，□ 表示0
    """
    bytes_per_row = (width + 7) // 8
    text = []
    for row in range(height):
        row_data = bitmap_data[row * bytes_per_row : (row + 1) * bytes_per_row]
        row_text = []
        for col in range(width):
            byte_idx = col // 8
            bit_idx = 7 - (col % 8)  # 高位在前
            if byte_idx < len(row_data):
                bit = (row_data[byte_idx] >> bit_idx) & 1
                row_text.append("■" if bit else "□")
        text.append("".join(row_text))
    return "\n".join(text)


def render_bitmap_image(
    bitmap_data: bytes, width: int, height: int, output_path: str, pixel_scale: int = 10
) -> None:
    """
    将位图数据渲染为 PNG 图片
    :param pixel_scale: 像素放大倍数（方便查看）
    """
    if Image is None:
        raise RuntimeError("Pillow 库未安装，无法生成图片！")

    # 创建空白图片（白色背景）
    img_width = width * pixel_scale
    img_height = height * pixel_scale
    img = Image.new("RGB", (img_width, img_height), "white")
    draw = ImageDraw.Draw(img)

    bytes_per_row = (width + 7) // 8

    # 绘制每个像素
    for row in range(height):
        row_data = bitmap_data[row * bytes_per_row : (row + 1) * bytes_per_row]
        for col in range(width):
            byte_idx = col // 8
            bit_idx = 7 - (col % 8)
            if byte_idx < len(row_data):
                bit = (row_data[byte_idx] >> bit_idx) & 1
                if bit:
                    # 绘制黑色像素块
                    x1 = col * pixel_scale
                    y1 = row * pixel_scale
                    x2 = x1 + pixel_scale
                    y2 = y1 + pixel_scale
                    draw.rectangle([x1, y1, x2, y2], fill="black")

    img.save(output_path)
    print(f"字符位图已保存到: {output_path}")


# -------------------------- 验证和统计函数 --------------------------
def validate_font_bin(
    font_name: str, font_data: bytes, metrics: List[Dict], char_count: int
) -> bool:
    """
    验证 bin 文件完整性
    :return: 验证通过返回 True，否则 False
    """
    print(f"\n=== 验证 {font_name} 字体 bin 文件 ===")
    valid = True

    for char_idx in range(char_count):
        metrics_item = metrics[char_idx]
        offset = metrics_item["offset"]
        width = metrics_item["width"]
        height = metrics_item["height"]

        bytes_per_row = (width + 7) // 8
        data_len = bytes_per_row * height

        # 检查偏移是否越界
        if offset > len(font_data):
            print(
                f"❌ 字符索引 {char_idx} (字符: {result['chars'][char_idx]})：偏移 {offset} 超过文件长度 {len(font_data)}"
            )
            valid = False
            continue

        # 检查数据长度是否越界
        if offset + data_len > len(font_data):
            print(
                f"❌ 字符索引 {char_idx} (字符: {result['chars'][char_idx]})：数据长度越界 (偏移+长度={offset+data_len} > 文件长度={len(font_data)})"
            )
            valid = False
            continue

        # 检查宽度/高度是否合法
        if width <= 0 or height <= 0:
            print(
                f"❌ 字符索引 {char_idx} (字符: {result['chars'][char_idx]})：非法尺寸 (宽={width}, 高={height})"
            )
            valid = False

    if valid:
        print(f"✅ 验证通过！共检查 {char_count} 个字符")
    return valid


def get_font_stats(
    font_name: str, font_data: bytes, metrics: List[Dict], char_count: int
) -> Dict:
    """
    获取字体统计信息
    """
    stats = {
        "font_name": font_name,
        "file_size": len(font_data),
        "char_count": char_count,
        "avg_char_size": 0,
        "max_char_size": 0,
        "min_char_size": float("inf"),
        "total_char_data_size": 0,
    }

    char_sizes = []
    for char_idx in range(char_count):
        metrics_item = metrics[char_idx]
        width = metrics_item["width"]
        height = metrics_item["height"]
        bytes_per_row = (width + 7) // 8
        data_len = bytes_per_row * height
        char_sizes.append(data_len)
        stats["total_char_data_size"] += data_len

        if data_len > stats["max_char_size"]:
            stats["max_char_size"] = data_len
        if data_len < stats["min_char_size"]:
            stats["min_char_size"] = data_len

    if char_count > 0:
        stats["avg_char_size"] = stats["total_char_data_size"] / char_count

    # 打印统计信息
    print(f"\n=== {font_name} 字体统计信息 ===")
    print(f"文件总大小: {stats['file_size']} 字节 ({stats['file_size']/1024:.2f} KB)")
    print(f"字符总数: {stats['char_count']}")
    print(f"字符数据总大小: {stats['total_char_data_size']} 字节")
    print(f"平均字符大小: {stats['avg_char_size']:.2f} 字节/字符")
    print(f"最大字符大小: {stats['max_char_size']} 字节")
    print(f"最小字符大小: {stats['min_char_size']} 字节")
    print(f"数据占比: {stats['total_char_data_size']/stats['file_size']*100:.2f}%")

    return stats


# -------------------------- 主函数 --------------------------
def main():
    parser = argparse.ArgumentParser(description="字体 bin 文件调试工具")
    parser.add_argument("--rs-file", required=True, help="generated_fonts.rs 文件路径")
    parser.add_argument("--bin-dir", required=True, help="字体 bin 文件所在目录")
    parser.add_argument(
        "command",
        choices=[
            "list-chars",  # 列出所有字符
            "show-metrics",  # 显示字符度量参数
            "render-char",  # 渲染字符位图
            "validate",  # 验证 bin 文件
            "stats",  # 显示统计信息
        ],
        help="调试命令",
    )
    parser.add_argument("--font-size", help="字体尺寸名称 (Small/Medium/Large)")
    parser.add_argument("--char", help="要操作的字符")
    parser.add_argument("--output", default="glyph.png", help="渲染图片输出路径")
    parser.add_argument("--scale", type=int, default=10, help="图片像素放大倍数")

    args = parser.parse_args()

    # 1. 解析 Rust 文件
    try:
        global result
        result = parse_generated_fonts_rs(args.rs_file)
        print(f"✅ 成功解析 Rust 文件：")
        print(f"   - 字符总数: {len(result['chars'])}")
        print(f"   - 缺失字符数: {len(result['missing_chars'])}")
        print(f"   - 支持的字体尺寸: {', '.join(result['font_sizes'].keys())}")
    except Exception as e:
        print(f"❌ 解析 Rust 文件失败: {e}")
        sys.exit(1)

    # 2. 处理不同命令
    if args.command == "list-chars":
        print("\n=== 共享字符表 ===")
        # 按每行20个字符显示
        chars_per_line = 20
        for i in range(0, len(result["chars"]), chars_per_line):
            line_chars = result["chars"][i : i + chars_per_line]
            print(f"  {''.join(line_chars)}")

        if result["missing_chars"]:
            print(f"\n=== 缺失字符 ===")
            print(f"  {''.join(result['missing_chars'])}")

    elif args.command in ["show-metrics", "render-char", "validate", "stats"]:
        # 检查字体尺寸参数
        if not args.font_size:
            print("❌ 请指定 --font-size 参数 (如 Small/Medium/Large)")
            sys.exit(1)

        font_name = args.font_size.capitalize()
        if font_name not in result["font_sizes"]:
            print(f"❌ 不支持的字体尺寸: {args.font_size}")
            print(f"   支持的尺寸: {', '.join(result['font_sizes'].keys())}")
            sys.exit(1)

        # 读取对应的 bin 文件
        bin_file_name = f"generated_{font_name.lower()}_font.bin"
        bin_file_path = os.path.join(args.bin_dir, bin_file_name)
        try:
            font_data = read_font_bin(bin_file_path)
            print(
                f"✅ 成功读取 bin 文件: {bin_file_path} (大小: {len(font_data)} 字节)"
            )
        except Exception as e:
            print(f"❌ 读取 bin 文件失败: {e}")
            sys.exit(1)

        # 获取该字体的度量参数
        metrics = result["metrics"].get(font_name)
        if not metrics:
            print(f"❌ 未找到 {font_name} 字体的度量参数")
            sys.exit(1)

        # 处理具体命令
        if args.command == "show-metrics":
            # 检查字符参数
            if not args.char:
                print("❌ 请指定 --char 参数")
                sys.exit(1)

            char = args.char
            if len(char) != 1:
                print("❌ --char 参数必须是单个字符")
                sys.exit(1)

            # 查找字符索引
            try:
                char_idx = result["chars"].index(char)
            except ValueError:
                print(f"❌ 字符 '{char}' 不在字符表中")
                sys.exit(1)

            # 显示度量参数
            char_metrics = metrics[char_idx]
            print(
                f"\n=== {font_name} 字体 - 字符 '{char}' (U+{ord(char):04X}) 度量参数 ==="
            )
            print(f"  偏移 (offset): {char_metrics['offset']} 字节")
            print(f"  宽度 (width): {char_metrics['width']} 像素")
            print(f"  高度 (height): {char_metrics['height']} 像素")
            print(f"  水平偏移 (bearing_x): {char_metrics['bearing_x']} 像素")
            print(f"  垂直偏移 (bearing_y): {char_metrics['bearing_y']} 像素")
            print(f"  水平步长 (advance_x): {char_metrics['advance_x']} 像素")

        elif args.command == "render-char":
            # 检查字符参数
            if not args.char:
                print("❌ 请指定 --char 参数")
                sys.exit(1)

            char = args.char
            if len(char) != 1:
                print("❌ --char 参数必须是单个字符")
                sys.exit(1)

            # 查找字符索引
            try:
                char_idx = result["chars"].index(char)
            except ValueError:
                print(f"❌ 字符 '{char}' 不在字符表中")
                sys.exit(1)

            # 获取位图数据
            try:
                bitmap_data, width, height = get_glyph_bitmap_data(
                    font_data, metrics, char_idx
                )
            except ValueError as e:
                print(f"❌ 获取位图数据失败: {e}")
                sys.exit(1)

            # 显示文本形式的位图
            print(
                f"\n=== {font_name} 字体 - 字符 '{char}' (U+{ord(char):04X}) 位图 ==="
            )
            print(f"  尺寸: {width}x{height} 像素")
            print(render_bitmap_text(bitmap_data, width, height))

            # 生成图片（如果安装了Pillow）
            if Image:
                try:
                    render_bitmap_image(
                        bitmap_data, width, height, args.output, args.scale
                    )
                except Exception as e:
                    print(f"⚠️  生成图片失败: {e}")

        elif args.command == "validate":
            validate_font_bin(font_name, font_data, metrics, len(result["chars"]))

        elif args.command == "stats":
            get_font_stats(font_name, font_data, metrics, len(result["chars"]))


if __name__ == "__main__":
    main()
