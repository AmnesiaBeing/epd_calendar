#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
图标 bin 文件调试工具（最终适配版）
完全兼容你的 generated_icons.rs 格式
"""

import re
import argparse
import os
import sys
from typing import Dict, List, Optional, Tuple, Any

try:
    from PIL import Image, ImageDraw
except ImportError:
    print("警告：未安装 Pillow 库，图片渲染功能不可用！")
    print("安装命令：pip install pillow")
    Image = None


# -------------------------- 核心数据结构 --------------------------
class IconCategoryInfo:
    """图标分类信息"""

    def __init__(
        self,
        name: str,
        enum_name: str,
        width: int,
        height: int,
        count: int,
        bitmap_len: int,
    ):
        self.name = name  # 分类名（如 battery, network, time_digit）
        self.enum_name = enum_name  # 枚举名（如 BatteryIcon, NetworkIcon）
        self.width = width  # 图标宽度
        self.height = height  # 图标高度
        self.count = count  # 图标数量
        self.bitmap_len = bitmap_len  # 单个图标位图长度
        self.icons: List[IconInfo] = []  # 该分类下的图标列表


class IconInfo:
    """单个图标信息"""

    def __init__(self, variant_name: str, original_id: str, index: int):
        self.variant_name = variant_name  # Rust枚举变体名
        self.original_id = original_id  # 原始ID（文件名/天气图标码）
        self.index = index  # 在分类中的索引


class WeatherIconInfo:
    """天气图标信息"""

    def __init__(
        self, enum_name: str, width: int, height: int, count: int, bitmap_len: int
    ):
        self.enum_name = enum_name
        self.width = width
        self.height = height
        self.count = count
        self.bitmap_len = bitmap_len
        self.icons: List[IconInfo] = []


# -------------------------- 核心解析函数（完全适配版） --------------------------
def parse_generated_icons_rs(rs_file_path: str) -> Dict[str, Any]:
    """
    解析 generated_icons.rs 文件（完全适配你的格式）
    """
    if not os.path.exists(rs_file_path):
        raise FileNotFoundError(f"Rust 文件不存在: {rs_file_path}")

    with open(rs_file_path, "r", encoding="utf-8") as f:
        content = f.read()

    result = {"categories": {}, "weather": None, "icon_id_mapping": {}}

    # ========== 1. 提取所有分类的基础信息（从注释+枚举+常量） ==========
    # 第一步：从 "/// XXX图标枚举" 提取分类名 + 枚举名映射
    category_enum_map = {}  # 分类名 -> 枚举名
    enum_category_map = {}  # 枚举名 -> 分类名
    enum_comment_pattern = re.compile(
        r"///\s*(\w+)图标枚举\s*\n.*?#\[derive\(Copy,\s*Clone,\s*Debug,\s*PartialEq,\s*Eq\)\].*?\n.*?pub\s+enum\s+(\w+)\s*\{",
        re.MULTILINE | re.DOTALL,  # 添加DOTALL以跨行匹配
    )
    for match in enum_comment_pattern.finditer(content):
        category_name = match.group(1).strip()  # battery/network/time_digit
        enum_name = match.group(2).strip()  # BatteryIcon/NetworkIcon/TimeDigitIcon
        category_enum_map[category_name] = enum_name
        enum_category_map[enum_name] = category_name

    # 第二步：提取所有常量（尺寸/数量/位图长度）
    # 匹配尺寸常量（支持带下划线的分类名，如 TIME_DIGIT_ICON_SIZE）
    size_pattern = re.compile(
        r"pub const (\w+_ICON_SIZE):\s*Size\s*=\s*Size\s*\{\s*"
        r"width:\s*(\d+),\s*"
        r"height:\s*(\d+),\s*"
        r"\};"
    )
    size_consts = {}  # 分类名 -> (width, height)
    for match in size_pattern.finditer(content):
        const_prefix = match.group(1)  # BATTERY/NETWORK/TIME_DIGIT/WEATHER
        width = int(match.group(2))
        height = int(match.group(3))
        # 转换为小写分类名（BATTERY -> battery, TIME_DIGIT -> time_digit）
        category_name = const_prefix.lower()
        size_consts[category_name] = (width, height)

    # 匹配数量常量
    count_pattern = re.compile(r"pub const (\w+)_ICON_COUNT:\s*usize\s*=\s*(\d+);")
    count_consts = {}  # 分类名 -> count
    for match in count_pattern.finditer(content):
        const_prefix = match.group(1)
        count = int(match.group(2))
        category_name = const_prefix.lower()
        count_consts[category_name] = count

    # 匹配位图长度常量
    bitmap_len_pattern = re.compile(
        r"pub const (\w+)_ICON_BITMAP_LEN:\s*usize\s*=\s*(\d+);"
    )
    bitmap_len_consts = {}  # 分类名 -> bitmap_len
    for match in bitmap_len_pattern.finditer(content):
        const_prefix = match.group(1)
        bitmap_len = int(match.group(2))
        category_name = const_prefix.lower()
        bitmap_len_consts[category_name] = bitmap_len

    # ========== 2. 解析本地图标分类（battery/network/time_digit） ==========
    # 匹配所有本地图标枚举（排除 WeatherIcon）
    local_enum_pattern = re.compile(
        r"#\[derive\(Copy,\s*Clone,\s*Debug,\s*PartialEq,\s*Eq\)\]\s+pub enum (\w+)\s*\{\s*([\s\S]*?)\s*\}\s*(?=///|impl|pub const)"
    )
    for match in local_enum_pattern.finditer(content):
        enum_name = match.group(1)
        enum_body = match.group(2)

        # 跳过天气图标枚举
        if enum_name == "WeatherIcon":
            continue

        # 获取分类名
        category_name = enum_category_map.get(enum_name)
        if not category_name:
            # 从枚举名推断（BatteryIcon -> battery, TimeDigitIcon -> time_digit）
            category_name = enum_name.lower().replace("icon", "")
            # 处理 TimeDigitIcon -> time_digit（驼峰转下划线）
            category_name = re.sub(r"(?<!^)(?=[A-Z])", "_", category_name).lower()

        # 获取该分类的常量信息
        width, height = size_consts.get(category_name, (0, 0))
        count = count_consts.get(category_name, 0)
        bitmap_len = bitmap_len_consts.get(category_name, 0)

        # 创建分类信息
        category_info = IconCategoryInfo(
            name=category_name,
            enum_name=enum_name,
            width=width,
            height=height,
            count=count,
            bitmap_len=bitmap_len,
        )

        # 解析枚举变体（匹配：    Battery0, // battery-0）
        variant_pattern = re.compile(r"^\s*(\w+),\s*//\s*(\S+)$", re.MULTILINE)
        variants = variant_pattern.findall(enum_body)
        for idx, (variant_name, original_id) in enumerate(variants):
            icon_info = IconInfo(
                variant_name=variant_name.strip(),
                original_id=original_id.strip(),
                index=idx,
            )
            category_info.icons.append(icon_info)
            # 构建IconId映射（匹配 get_icon_data 中的格式：battery:battery-0）
            result["icon_id_mapping"][f"{category_name}:{original_id.strip()}"] = (
                category_name,
                variant_name.strip(),
            )

        # 添加到结果
        result["categories"][category_name] = category_info

    # ========== 3. 解析天气图标（WeatherIcon） ==========
    weather_enum_pattern = re.compile(
        r"#\[derive\(Copy,\s*Clone,\s*Debug,\s*PartialEq,\s*Eq\)\]\s+pub enum WeatherIcon\s*\{\s*([\s\S]*?)\s*\}\s*(?=impl)"
    )
    weather_match = weather_enum_pattern.search(content)
    if weather_match:
        enum_body = weather_match.group(1)

        # 获取天气图标常量
        width, height = size_consts.get("weather", (0, 0))
        count = count_consts.get("weather", 0)
        bitmap_len = bitmap_len_consts.get("weather", 0)

        weather_info = WeatherIconInfo(
            enum_name="WeatherIcon",
            width=width,
            height=height,
            count=count,
            bitmap_len=bitmap_len,
        )

        # 解析天气图标变体（匹配：    Icon100, // 100）
        variant_pattern = re.compile(r"^\s*(\w+),\s*//\s*(\S+)$", re.MULTILINE)
        variants = variant_pattern.findall(enum_body)
        for idx, (variant_name, original_id) in enumerate(variants):
            icon_info = IconInfo(
                variant_name=variant_name.strip(),
                original_id=original_id.strip(),
                index=idx,
            )
            weather_info.icons.append(icon_info)
            # 构建映射（支持 100、icon_100 两种格式）
            result["icon_id_mapping"][original_id.strip()] = (
                "weather",
                variant_name.strip(),
            )
            result["icon_id_mapping"][f"icon_{original_id.strip()}"] = (
                "weather",
                variant_name.strip(),
            )

        result["weather"] = weather_info

    return result


def read_icon_bin(bin_file_path: str) -> bytes:
    """读取图标 bin 文件"""
    if not os.path.exists(bin_file_path):
        raise FileNotFoundError(f"Bin 文件不存在: {bin_file_path}")
    with open(bin_file_path, "rb") as f:
        return f.read()


def get_icon_bitmap_data(bin_data: bytes, icon_index: int, bitmap_len: int) -> bytes:
    """从bin文件中提取单个图标的位图数据"""
    start = icon_index * bitmap_len
    end = start + bitmap_len
    if end > len(bin_data):
        raise ValueError(f"图标索引 {icon_index} 数据越界！")
    return bin_data[start:end]


# -------------------------- 位图渲染函数 --------------------------
def render_icon_bitmap_text(bitmap_data: bytes, width: int, height: int) -> str:
    """将图标位图数据渲染为文本形式（■=1，□=0）"""
    text = []
    total_pixels = width * height
    pixel_idx = 0

    for row in range(height):
        row_text = []
        for col in range(width):
            if pixel_idx >= total_pixels:
                break

            byte_idx = pixel_idx // 8
            bit_idx = 7 - (pixel_idx % 8)  # 高位在前

            if byte_idx < len(bitmap_data):
                bit = (bitmap_data[byte_idx] >> bit_idx) & 1
                row_text.append("■" if bit else "□")

            pixel_idx += 1
        text.append("".join(row_text))

    return "\n".join(text)


def render_icon_bitmap_image(
    bitmap_data: bytes, width: int, height: int, output_path: str, pixel_scale: int = 10
) -> None:
    """将图标位图数据渲染为 PNG 图片"""
    if Image is None:
        raise RuntimeError("Pillow 库未安装，无法生成图片！")

    # 创建空白图片（白色背景）
    img_width = width * pixel_scale
    img_height = height * pixel_scale
    img = Image.new("RGB", (img_width, img_height), "white")
    draw = ImageDraw.Draw(img)

    total_pixels = width * height
    pixel_idx = 0

    # 绘制每个像素
    for row in range(height):
        for col in range(width):
            if pixel_idx >= total_pixels:
                break

            byte_idx = pixel_idx // 8
            bit_idx = 7 - (pixel_idx % 8)

            if byte_idx < len(bitmap_data):
                bit = (bitmap_data[byte_idx] >> bit_idx) & 1
                if bit:
                    # 绘制黑色像素块
                    x1 = col * pixel_scale
                    y1 = row * pixel_scale
                    x2 = x1 + pixel_scale
                    y2 = y1 + pixel_scale
                    draw.rectangle([x1, y1, x2, y2], fill="black")

            pixel_idx += 1

    img.save(output_path)
    print(f"✅ 图标位图已保存到: {output_path}")


# -------------------------- 验证和统计函数 --------------------------
def validate_icon_bin(
    bin_name: str, bin_data: bytes, icon_count: int, bitmap_len: int
) -> bool:
    """验证图标 bin 文件完整性"""
    print(f"\n=== 验证 {bin_name} 图标 bin 文件 ===")
    valid = True

    # 检查总长度
    expected_total_len = icon_count * bitmap_len
    if len(bin_data) != expected_total_len:
        print(
            f"❌ 文件总长度不匹配：实际 {len(bin_data)} 字节，期望 {expected_total_len} 字节"
        )
        valid = False
    else:
        print(f"✅ 文件总长度验证通过")

    # 检查每个图标数据偏移
    for idx in range(icon_count):
        start = idx * bitmap_len
        end = start + bitmap_len
        if end > len(bin_data):
            print(
                f"❌ 图标索引 {idx}：数据偏移越界 (偏移+长度={end} > 文件长度={len(bin_data)})"
            )
            valid = False

    if valid:
        print(f"✅ 验证通过！共检查 {icon_count} 个图标")
    return valid


def get_icon_stats(
    bin_name: str,
    bin_data: bytes,
    icon_count: int,
    bitmap_len: int,
    width: int,
    height: int,
) -> Dict:
    """获取图标统计信息"""
    stats = {
        "bin_name": bin_name,
        "file_size": len(bin_data),
        "icon_count": icon_count,
        "icon_width": width,
        "icon_height": height,
        "single_icon_size": bitmap_len,
        "total_icon_data_size": icon_count * bitmap_len,
        "unused_bytes": len(bin_data) - (icon_count * bitmap_len),
    }

    # 打印统计信息
    print(f"\n=== {bin_name} 图标统计信息 ===")
    print(f"文件总大小: {stats['file_size']} 字节 ({stats['file_size']/1024:.2f} KB)")
    print(f"图标总数: {stats['icon_count']}")
    print(f"单个图标尺寸: {stats['icon_width']}x{stats['icon_height']} 像素")
    print(f"单个图标数据大小: {stats['single_icon_size']} 字节")
    print(f"图标数据总大小: {stats['total_icon_data_size']} 字节")
    print(f"未使用字节数: {stats['unused_bytes']} 字节")
    if stats["file_size"] > 0:
        print(
            f"数据利用率: {stats['total_icon_data_size']/stats['file_size']*100:.2f}%"
        )

    return stats


# -------------------------- 辅助函数 --------------------------
def find_icon_by_id(icon_id: str, parsed_data: Dict) -> Tuple[str, IconInfo, Any]:
    """根据图标ID查找图标信息"""
    # 直接匹配映射
    if icon_id in parsed_data["icon_id_mapping"]:
        icon_type, variant_name = parsed_data["icon_id_mapping"][icon_id]

        if icon_type == "weather":
            weather_info = parsed_data["weather"]
            if weather_info:
                for icon in weather_info.icons:
                    if icon.variant_name == variant_name:
                        return ("weather", icon, weather_info)
        else:
            category_info = parsed_data["categories"].get(icon_type)
            if category_info:
                for icon in category_info.icons:
                    if icon.variant_name == variant_name:
                        return (icon_type, icon, category_info)

    # 未找到
    raise ValueError(
        f"未找到图标 ID: {icon_id}\n可用的ID示例：\n  - battery:battery-0\n  - network:connected\n  - time_digit:digit_0\n  - 100 (天气图标)\n  - icon_100 (天气图标)"
    )


# -------------------------- 主函数 --------------------------
def main():
    parser = argparse.ArgumentParser(
        description="图标 bin 文件调试工具（适配你的格式）"
    )
    parser.add_argument("--rs-file", required=True, help="generated_icons.rs 文件路径")
    parser.add_argument("--bin-dir", required=True, help="图标 bin 文件所在目录")
    parser.add_argument(
        "command",
        choices=[
            "list-icons",  # 列出所有图标
            "show-info",  # 显示图标详细信息
            "render-icon",  # 渲染图标位图
            "validate",  # 验证 bin 文件
            "stats",  # 显示统计信息
        ],
        help="调试命令",
    )
    parser.add_argument(
        "--category", help="图标分类名 (battery/network/time_digit/weather)"
    )
    parser.add_argument("--icon-id", help="要操作的图标ID（如 battery:battery-0、100）")
    parser.add_argument("--output", default="icon.png", help="渲染图片输出路径")
    parser.add_argument("--scale", type=int, default=10, help="图片像素放大倍数")

    args = parser.parse_args()

    # 1. 解析 Rust 文件
    try:
        parsed_data = parse_generated_icons_rs(args.rs_file)
        print(f"✅ 成功解析 Rust 文件：")
        print(
            f"   - 本地图标分类: {', '.join(parsed_data['categories'].keys()) if parsed_data['categories'] else '无'}"
        )
        if parsed_data["weather"]:
            print(f"   - 天气图标数量: {len(parsed_data['weather'].icons)}")
        else:
            print(f"   - 无天气图标")
    except Exception as e:
        print(f"❌ 解析 Rust 文件失败: {e}")
        import traceback

        traceback.print_exc()
        sys.exit(1)

    # 2. 处理不同命令
    if args.command == "list-icons":
        print("\n=== 所有图标列表 ===")

        # 列出本地图标分类
        for cat_name, cat_info in parsed_data["categories"].items():
            print(
                f"\n【{cat_name} 分类】 (尺寸: {cat_info.width}x{cat_info.height}, 数量: {len(cat_info.icons)})"
            )
            for icon in cat_info.icons:
                print(
                    f"  - {icon.original_id} (变体名: {icon.variant_name}, 索引: {icon.index})"
                )

        # 列出天气图标（只显示前20个，避免输出过长）
        if parsed_data["weather"]:
            weather_info = parsed_data["weather"]
            print(
                f"\n【weather 分类】 (尺寸: {weather_info.width}x{weather_info.height}, 数量: {len(weather_info.icons)})"
            )
            print(f"  前20个天气图标：")
            for i, icon in enumerate(weather_info.icons[:20]):
                print(
                    f"  - {icon.original_id} (变体名: {icon.variant_name}, 索引: {icon.index})"
                )
            if len(weather_info.icons) > 20:
                print(f"  ... 共 {len(weather_info.icons)} 个天气图标（仅显示前20个）")

    elif args.command == "show-info":
        if not args.icon_id:
            print(
                "❌ 请指定 --icon-id 参数（示例：battery:battery-0、network:connected、time_digit:digit_0、100）"
            )
            sys.exit(1)

        try:
            icon_type, icon_info, parent_info = find_icon_by_id(
                args.icon_id, parsed_data
            )
        except ValueError as e:
            print(f"❌ {e}")
            sys.exit(1)

        # 显示图标信息
        print(f"\n=== 图标详细信息 ===")
        print(f"  ID: {args.icon_id}")
        print(f"  类型: {icon_type}")
        print(f"  变体名: {icon_info.variant_name}")
        print(f"  原始ID: {icon_info.original_id}")
        print(f"  索引: {icon_info.index}")
        print(f"  尺寸: {parent_info.width}x{parent_info.height} 像素")
        print(f"  单个图标数据长度: {parent_info.bitmap_len} 字节")
        print(f"  数据偏移: {icon_info.index * parent_info.bitmap_len} 字节")

    elif args.command == "render-icon":
        if not args.icon_id:
            print(
                "❌ 请指定 --icon-id 参数（示例：battery:battery-0、network:connected、time_digit:digit_0、100）"
            )
            sys.exit(1)

        try:
            icon_type, icon_info, parent_info = find_icon_by_id(
                args.icon_id, parsed_data
            )
        except ValueError as e:
            print(f"❌ {e}")
            sys.exit(1)

        # 读取对应的 bin 文件
        if icon_type == "weather":
            bin_file_name = "generated_weather_icons.bin"
        else:
            bin_file_name = f"generated_{icon_type}_icons.bin"

        bin_file_path = os.path.join(args.bin_dir, bin_file_name)
        try:
            bin_data = read_icon_bin(bin_file_path)
            print(f"✅ 成功读取 bin 文件: {bin_file_path} (大小: {len(bin_data)} 字节)")
        except Exception as e:
            print(f"❌ 读取 bin 文件失败: {e}")
            sys.exit(1)

        # 提取图标位图数据
        try:
            bitmap_data = get_icon_bitmap_data(
                bin_data, icon_info.index, parent_info.bitmap_len
            )
        except ValueError as e:
            print(f"❌ 获取位图数据失败: {e}")
            sys.exit(1)

        # 显示文本形式的位图
        print(
            f"\n=== 图标 '{args.icon_id}' 位图 ({parent_info.width}x{parent_info.height}) ==="
        )
        print(
            render_icon_bitmap_text(bitmap_data, parent_info.width, parent_info.height)
        )

        # 生成图片（如果安装了Pillow）
        if Image:
            try:
                render_icon_bitmap_image(
                    bitmap_data,
                    parent_info.width,
                    parent_info.height,
                    args.output,
                    args.scale,
                )
            except Exception as e:
                print(f"⚠️  生成图片失败: {e}")

    elif args.command == "validate":
        if not args.category:
            print("❌ 请指定 --category 参数 (battery/network/time_digit/weather)")
            sys.exit(1)

        category = args.category.lower()
        if category == "weather":
            if not parsed_data["weather"]:
                print("❌ 未找到天气图标信息")
                sys.exit(1)
            weather_info = parsed_data["weather"]
            bin_file_name = "generated_weather_icons.bin"
            bin_file_path = os.path.join(args.bin_dir, bin_file_name)

            try:
                bin_data = read_icon_bin(bin_file_path)
                print(
                    f"✅ 成功读取 bin 文件: {bin_file_path} (大小: {len(bin_data)} 字节)"
                )
            except Exception as e:
                print(f"❌ 读取 bin 文件失败: {e}")
                sys.exit(1)

            validate_icon_bin(
                "weather", bin_data, len(weather_info.icons), weather_info.bitmap_len
            )
        else:
            category_info = parsed_data["categories"].get(category)
            if not category_info:
                print(f"❌ 未找到分类: {category}")
                print(f"   支持的分类: {', '.join(parsed_data['categories'].keys())}")
                sys.exit(1)

            bin_file_name = f"generated_{category}_icons.bin"
            bin_file_path = os.path.join(args.bin_dir, bin_file_name)

            try:
                bin_data = read_icon_bin(bin_file_path)
                print(
                    f"✅ 成功读取 bin 文件: {bin_file_path} (大小: {len(bin_data)} 字节)"
                )
            except Exception as e:
                print(f"❌ 读取 bin 文件失败: {e}")
                sys.exit(1)

            validate_icon_bin(
                category, bin_data, len(category_info.icons), category_info.bitmap_len
            )

    elif args.command == "stats":
        if not args.category:
            print("❌ 请指定 --category 参数 (battery/network/time_digit/weather)")
            sys.exit(1)

        category = args.category.lower()
        if category == "weather":
            if not parsed_data["weather"]:
                print("❌ 未找到天气图标信息")
                sys.exit(1)
            weather_info = parsed_data["weather"]
            bin_file_name = "generated_weather_icons.bin"
            bin_file_path = os.path.join(args.bin_dir, bin_file_name)

            try:
                bin_data = read_icon_bin(bin_file_path)
                print(
                    f"✅ 成功读取 bin 文件: {bin_file_path} (大小: {len(bin_data)} 字节)"
                )
            except Exception as e:
                print(f"❌ 读取 bin 文件失败: {e}")
                sys.exit(1)

            get_icon_stats(
                "weather",
                bin_data,
                len(weather_info.icons),
                weather_info.bitmap_len,
                weather_info.width,
                weather_info.height,
            )
        else:
            category_info = parsed_data["categories"].get(category)
            if not category_info:
                print(f"❌ 未找到分类: {category}")
                print(f"   支持的分类: {', '.join(parsed_data['categories'].keys())}")
                sys.exit(1)

            bin_file_name = f"generated_{category}_icons.bin"
            bin_file_path = os.path.join(args.bin_dir, bin_file_name)

            try:
                bin_data = read_icon_bin(bin_file_path)
                print(
                    f"✅ 成功读取 bin 文件: {bin_file_path} (大小: {len(bin_data)} 字节)"
                )
            except Exception as e:
                print(f"❌ 读取 bin 文件失败: {e}")
                sys.exit(1)

            get_icon_stats(
                category,
                bin_data,
                len(category_info.icons),
                category_info.bitmap_len,
                category_info.width,
                category_info.height,
            )


if __name__ == "__main__":
    main()
