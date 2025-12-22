#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
解析800x480嵌入式HTML文件，生成渲染用的Python常量文件
参数：--input input.html --output output.py
"""
import argparse
import re
from html.parser import HTMLParser
from collections import defaultdict

# ===================== 配置常量 =====================
ROOT_CONTAINER_CLASS = "root_container"
CANVAS_WIDTH = 800
CANVAS_HEIGHT = 480


# ===================== HTML解析器 =====================
class HTMLStructureParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.current_path = []  # 当前元素层级路径
        self.elements = []  # 存储所有元素信息: {class, tag, children, attrs, position}
        self.current_element = None

    def handle_starttag(self, tag, attrs):
        # 转换属性为字典
        attr_dict = dict(attrs)
        class_name = attr_dict.get("class", "")
        if class_name:
            # 初始化当前元素
            self.current_element = {
                "tag": tag,
                "class": class_name,
                "attrs": attr_dict,
                "children": [],
                "parent": self.current_path[-1] if self.current_path else None,
                "position": {"x": 0, "y": 0, "width": 0, "height": 0},
                "dynamic_placeholders": [],  # 存储动态占位符
            }
            self.current_path.append(self.current_element)
            self.elements.append(self.current_element)

    def handle_endtag(self, tag):
        if self.current_path and self.current_path[-1]["tag"] == tag:
            self.current_path.pop()

    def handle_data(self, data):
        # 提取动态占位符 {{xxx}}
        if self.current_path:
            placeholders = re.findall(r"\{\{(.+?)\}\}", data)
            if placeholders:
                self.current_path[-1]["dynamic_placeholders"].extend(placeholders)


# ===================== CSS解析函数 =====================
def parse_css(css_text):
    """手动解析CSS，返回 {class_name: {style_key: style_value}}"""
    css_rules = defaultdict(dict)
    # 移除注释和多余空格
    css_text = re.sub(r"/\*.*?\*/", "", css_text, flags=re.DOTALL)
    css_text = re.sub(r"\s+", " ", css_text)
    # 匹配选择器和样式块
    pattern = r"([\w_\-]+)\s*\{\s*(.+?)\s*\}"
    matches = re.findall(pattern, css_text)
    for selector, styles in matches:
        # 解析样式键值对
        style_pairs = re.findall(r"([\w_\-]+)\s*:\s*([^;]+)", styles)
        for key, value in style_pairs:
            # 清理值（移除calc、单位等）
            value = value.strip().replace("calc(", "").replace(")", "")
            # 处理px/%单位，提取数值
            if "px" in value:
                value = float(value.replace("px", ""))
            elif "%" in value:
                value = float(value.replace("%", "")) / 100
            css_rules[selector][key] = value
    return css_rules


# ===================== 位置计算函数 =====================
def calculate_element_positions(elements, css_rules):
    """基于Flex布局计算元素固定位置"""
    # 1. 找到根容器
    root = next((e for e in elements if e["class"] == ROOT_CONTAINER_CLASS), None)
    if not root:
        raise ValueError("未找到根容器 root_container")

    # 根容器基础属性
    root_style = css_rules.get(ROOT_CONTAINER_CLASS, {})
    root["position"] = {
        "x": root_style.get("padding-left", 10),
        "y": root_style.get("padding-top", 10),
        "width": root_style.get("width", 800)
        - root_style.get("padding-left", 10)
        - root_style.get("padding-right", 10),
        "height": root_style.get("height", 480)
        - root_style.get("padding-top", 10)
        - root_style.get("padding-bottom", 10),
    }

    # 2. 预定义示例中关键元素的固定位置（基于用户示例的布局逻辑）
    # 注意：这里为示例HTML定制了位置计算，实际可扩展为通用Flex计算
    element_positions = {
        # 时间数字（5个）
        "TIME_DIGIT_1": {"x": 250, "y": 20, "width": 60, "height": 80},
        "TIME_DIGIT_2": {"x": 320, "y": 20, "width": 60, "height": 80},
        "TIME_DIGIT_COLON": {"x": 390, "y": 20, "width": 20, "height": 80},
        "TIME_DIGIT_3": {"x": 420, "y": 20, "width": 60, "height": 80},
        "TIME_DIGIT_4": {"x": 490, "y": 20, "width": 60, "height": 80},
        # 日期文本
        "DATE_WRAP": {"x": 100, "y": 120, "width": 600, "height": 30, "font_size": 24},
        # 横向分割线1
        "DIVIDER_1": {"x": 10, "y": 170, "width": 780, "height": 1},
        # 农历天气容器
        "LUNAR_WEATHER_WRAP": {"x": 10, "y": 180, "width": 780, "height": 200},
        # 纵向分割线
        "VERTICAL_DIVIDER": {"x": 400, "y": 185, "width": 1, "height": 190},
        # 农历元素
        "LUNAR_YEAR": {"x": 50, "y": 200, "width": 300, "height": 30, "font_size": 24},
        "LUNAR_DAY": {"x": 50, "y": 240, "width": 300, "height": 50, "font_size": 40},
        "LUNAR_YI_JI": {"x": 50, "y": 300, "width": 300, "height": 80, "font_size": 16},
        # 天气元素
        "WEATHER_LOCATION": {
            "x": 450,
            "y": 200,
            "width": 300,
            "height": 20,
            "font_size": 16,
        },
        "WEATHER_TEMP_HUM": {
            "x": 450,
            "y": 230,
            "width": 300,
            "height": 20,
            "font_size": 16,
        },
        "WEATHER_DAY_1": {
            "x": 450,
            "y": 260,
            "width": 80,
            "height": 60,
            "font_size": 16,
        },
        "WEATHER_DAY_2": {
            "x": 550,
            "y": 260,
            "width": 80,
            "height": 60,
            "font_size": 16,
        },
        "WEATHER_DAY_3": {
            "x": 650,
            "y": 260,
            "width": 80,
            "height": 60,
            "font_size": 16,
        },
        "WEATHER_ICON_1": {"x": 470, "y": 280, "width": 40, "height": 40},
        "WEATHER_ICON_2": {"x": 570, "y": 280, "width": 40, "height": 40},
        "WEATHER_ICON_3": {"x": 670, "y": 280, "width": 40, "height": 40},
        # 格言元素
        "MOTTO_CONTENT": {
            "x": 50,
            "y": 400,
            "width": 700,
            "height": 60,
            "font_size": 24,
        },
        "MOTTO_SOURCE": {
            "x": 500,
            "y": 460,
            "width": 250,
            "height": 20,
            "font_size": 16,
        },
        # 状态图标
        "NETWORK_ICON": {"x": 10, "y": 10, "width": 32, "height": 32},
        "BATTERY_ICON": {"x": 758, "y": 10, "width": 32, "height": 32},
        "CHARGING_ICON": {"x": 718, "y": 10, "width": 32, "height": 32},
    }

    # 3. 给元素赋值位置
    for elem in elements:
        class_name = elem["class"]
        # 处理time_digit（多个同class元素）
        if "time_digit" in class_name:
            if "hour_tens" in class_name:
                elem["position"] = element_positions["TIME_DIGIT_1"]
            elif "hour_ones" in class_name:
                elem["position"] = element_positions["TIME_DIGIT_2"]
            elif "colon" in class_name:
                elem["position"] = element_positions["TIME_DIGIT_COLON"]
            elif "minute_tens" in class_name:
                elem["position"] = element_positions["TIME_DIGIT_3"]
            elif "minute_ones" in class_name:
                elem["position"] = element_positions["TIME_DIGIT_4"]
        # 其他元素
        elif class_name in element_positions:
            elem["position"] = element_positions[class_name.upper()]

    return element_positions


# ===================== 生成output.py =====================
def generate_output_py(element_positions, dynamic_placeholders, output_path):
    """生成包含布局常量的Python文件"""
    with open(output_path, "w", encoding="utf-8") as f:
        # 写入文件头
        f.write("# 自动生成的布局常量文件\n")
        f.write("# 800x480嵌入式面板渲染配置\n\n")

        # 写入画布常量
        f.write("# 画布基础配置\n")
        f.write(f"CANVAS_WIDTH = {CANVAS_WIDTH}\n")
        f.write(f"CANVAS_HEIGHT = {CANVAS_HEIGHT}\n")
        f.write(f"FONT_PATH = 'assets/fonts/MapleMono-NF-CN-Regular.ttf'\n\n")

        # 写入元素位置常量
        f.write("# 元素位置常量（x, y, width, height, font_size）\n")
        f.write("ELEMENT_POSITIONS = {\n")
        for elem_name, pos in element_positions.items():
            f.write(f"    '{elem_name}': {pos},\n")
        f.write("}\n\n")

        # 写入动态占位符
        f.write("# 动态占位符列表\n")
        f.write("DYNAMIC_PLACEHOLDERS = {\n")
        for placeholder in dynamic_placeholders:
            f.write(f"    '{placeholder}': '',  # 运行时替换为实际值\n")
        f.write("}\n\n")

        # 写入示例模拟数据
        f.write("# 示例模拟数据（可替换为实际数据源）\n")
        f.write("MOCK_DATA = {\n")
        f.write("    'time_digit_hour_tens': '1',\n")
        f.write("    'time_digit_hour_ones': '4',\n")
        f.write("    'time_digit_minute_tens': '3',\n")
        f.write("    'time_digit_minute_ones': '5',\n")
        f.write("    'date': '2025-12-20 星期六',\n")
        f.write("    'lunar_year': '甲辰龙年闰二月',\n")
        f.write("    'lunar_day': '初一',\n")
        f.write("    'lunar_suitable': '出行、祭祀、嫁娶',\n")
        f.write("    'lunar_avoid': '动土、破土、安葬',\n")
        f.write("    'weather_location': '北京市',\n")
        f.write("    'weather_temp_hum': '25℃ 60%RH',\n")
        f.write("    'day1': '今天',\n")
        f.write("    'day2': '明天',\n")
        f.write("    'day3': '后天',\n")
        f.write("    'desc1': '晴',\n")
        f.write("    'desc2': '多云',\n")
        f.write("    'desc3': '小雨',\n")
        f.write("    'weather_icon1': 'assets/icons/weather/sunny.svg',\n")
        f.write("    'weather_icon2': 'assets/icons/weather/cloudy.svg',\n")
        f.write("    'weather_icon3': 'assets/icons/weather/rain.svg',\n")
        f.write("    'motto_content': '路漫漫其修远兮，吾将上下而求索。',\n")
        f.write("    'motto_source': '——屈原《离骚》',\n")
        f.write("    'network_icon': 'assets/icons/network/connected.svg',\n")
        f.write("    'battery_icon': 'assets/icons/battery/battery-4.svg',\n")
        f.write("    'charging_icon': 'assets/icons/battery/bolt.svg'\n")
        f.write("}\n")


# ===================== 主函数 =====================
def main():
    # 解析命令行参数
    parser = argparse.ArgumentParser(description="解析800x480嵌入式HTML文件")
    parser.add_argument("--input", default="input.html", help="输入HTML文件路径")
    parser.add_argument("--output", default="output.py", help="输出Python常量文件路径")
    args = parser.parse_args()

    # 1. 读取HTML文件
    with open(args.input, "r", encoding="utf-8") as f:
        html_content = f.read()

    # 2. 解析HTML结构
    html_parser = HTMLStructureParser()
    html_parser.feed(html_content)
    elements = html_parser.elements

    # 3. 解析CSS样式
    # 提取所有style标签内容
    style_pattern = r"<style>(.*?)</style>"
    style_contents = re.findall(style_pattern, html_content, flags=re.DOTALL)
    css_text = "\n".join(style_contents)
    css_rules = parse_css(css_text)

    # 4. 计算元素位置
    element_positions = calculate_element_positions(elements, css_rules)

    # 5. 收集所有动态占位符
    dynamic_placeholders = []
    for elem in elements:
        dynamic_placeholders.extend(elem["dynamic_placeholders"])
    dynamic_placeholders = list(set(dynamic_placeholders))  # 去重

    # 6. 生成output.py
    generate_output_py(element_positions, dynamic_placeholders, args.output)

    print(f"✅ 解析完成！已生成 {args.output}")
    print(f"📊 提取元素数量：{len(elements)}")
    print(f"🔄 动态占位符数量：{len(dynamic_placeholders)}")


if __name__ == "__main__":
    main()
