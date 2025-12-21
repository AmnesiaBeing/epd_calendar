#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
解析800x480嵌入式HTML（支持id选择器+{{}}占位符），生成PIL渲染代码
"""
import argparse
import re
from bs4 import BeautifulSoup
import cssutils

# 全局配置
SCREEN_WIDTH = 800
SCREEN_HEIGHT = 480
MAX_NEST_LEVEL = 10


def parse_css(css_text):
    """解析CSS文本，生成选择器（id/class）到样式属性的映射"""
    css_dict = {"class": {}, "id": {}}
    sheet = cssutils.parseString(css_text)

    for rule in sheet:
        if rule.type == rule.STYLE_RULE:
            # 提取选择器（支持id和class选择器）
            for selector in rule.selectorList:
                selector_text = selector.text.strip()
                # 处理id选择器
                if selector_text.startswith("#"):
                    elem_id = selector_text[1:]
                    style = {
                        prop.name.lower(): prop.value.strip() for prop in rule.style
                    }
                    css_dict["id"][elem_id] = style
                # 处理类选择器
                elif selector_text.startswith("."):
                    class_name = selector_text[1:]
                    style = {
                        prop.name.lower(): prop.value.strip() for prop in rule.style
                    }
                    css_dict["class"][class_name] = style
    return css_dict


def extract_css_from_html(html_content):
    """从HTML中提取style标签内的CSS内容"""
    soup = BeautifulSoup(html_content, "html.parser")
    style_tags = soup.find_all("style")
    css_text = "\n".join([tag.text for tag in style_tags])
    return css_text


def get_element_style(element, css_dict):
    """获取元素的合并样式（id + class）"""
    style = {}
    # 优先获取id样式
    elem_id = element.get("id", "")
    if elem_id and elem_id in css_dict["id"]:
        style.update(css_dict["id"][elem_id])
    # 补充class样式
    class_list = element.get("class", [])
    for cls in class_list:
        if cls in css_dict["class"]:
            style.update(css_dict["class"][cls])
    return style


def parse_dimension(value, parent_size=None, default=0):
    """解析尺寸值（px/%/calc），返回数值"""
    if not value:
        return default

    value = str(value).strip()
    # 处理calc表达式（仅支持 calc(100% - Npx) 格式）
    if value.startswith("calc("):
        calc_match = re.search(r"calc\((\d+)% - (\d+)px\)", value)
        if calc_match and parent_size:
            percent = int(calc_match.group(1))
            px = int(calc_match.group(2))
            return int(parent_size * percent / 100) - px

    # 处理像素值
    if value.endswith("px"):
        return int(value[:-2].strip())
    # 处理百分比
    elif value.endswith("%"):
        if parent_size is None:
            return 0
        percent = float(value[:-1].strip())
        return int(parent_size * percent / 100)
    # 纯数字（默认px）
    elif value.isdigit():
        return int(value)
    # 处理flex相关（暂时返回标记）
    elif value == "1" or value == "0":
        return int(value)
    return default


def extract_placeholders(text):
    """提取{{xxx}}格式的占位符"""
    if not text:
        return []
    return re.findall(r"\{\{(.*?)\}\}", str(text))


def generate_python_code(element_tree, css_dict, font_path):
    """生成PIL渲染的Python代码"""
    code_lines = [
        "#!/usr/bin/env python3",
        "# -*- coding: utf-8 -*-",
        '"""',
        "自动生成的800x480信息面板布局渲染代码",
        "使用PIL库渲染布局为PNG图片",
        '"""',
        "from PIL import Image, ImageDraw, ImageFont",
        "import os",
        "",
        "# 屏幕尺寸配置",
        f"SCREEN_WIDTH = {SCREEN_WIDTH}",
        f"SCREEN_HEIGHT = {SCREEN_HEIGHT}",
        "",
        "# 模拟数据（可根据实际需求修改）",
        "mock_data = {",
        "    # 基础信息",
        "    'time': '14:35',",
        "    'date': '2025-12-20 星期六',",
        "    # 农历信息",
        "    'lunar_year': '甲辰龙年闰二月',",
        "    'lunar_day': '初一',",
        "    'lunar_suitable': '出行、祭祀、嫁娶',",
        "    'lunar_avoid': '动土、破土、安葬',",
        "    # 天气信息",
        "    'weather_location': '北京市',",
        "    'weather_temp_hum': '25℃ 60%RH',",
        "    'weather_3days': [",
        "        {'day': '今天', 'icon': 'sunny', 'desc': '晴'},",
        "        {'day': '明天', 'icon': 'cloudy', 'desc': '多云'},",
        "        {'day': '后天', 'icon': 'rain', 'desc': '小雨'},",
        "    ],",
        "    # 格言信息",
        "    'motto_content': '路漫漫其修远兮，吾将上下而求索。亦余心之所善兮，虽九死其犹未悔。',",
        "    'motto_source': '——屈原《离骚》',",
        "    # 状态图标",
        "    'network': 'connected',",
        "    'battery': '4',",
        "    'charging': False,",
        "}",
        "",
        "# 初始化画布",
        "def init_canvas():",
        '    """创建800x480的白色画布"""',
        "    img = Image.new('RGB', (SCREEN_WIDTH, SCREEN_HEIGHT), 'white')",
        "    draw = ImageDraw.Draw(img)",
        "    return img, draw",
        "",
        "# 加载字体",
        "def load_font(font_size):",
        '    """加载指定大小的嵌入式字体"""',
        "    try:",
        f"        font = ImageFont.truetype('{font_path}', font_size)",
        "    except Exception as e:",
        '        print(f"字体加载失败，使用默认字体: {e}")',
        "        font = ImageFont.load_default()",
        "    return font",
        "",
        "# 计算文本行数和位置",
        "def calculate_text_layout(draw, text, font, max_width, line_height):",
        '    """计算多行文本的布局（自动换行）"""',
        "    lines = []",
        "    current_line = ''",
        "    current_width = 0",
        "",
        "    for char in text:",
        "        char_width = draw.textlength(char, font=font)",
        "        if current_width + char_width > max_width or char == '\\n':",
        "            lines.append(current_line)",
        "            current_line = char if char != '\\n' else ''",
        "            current_width = char_width if char != '\\n' else 0",
        "        else:",
        "            current_line += char",
        "            current_width += char_width",
        "    if current_line:",
        "        lines.append(current_line)",
        "",
        "    return lines, len(lines) * line_height",
        "",
        "# 计算元素位置和尺寸",
        "def calculate_layout(draw):",
        '    """计算所有元素的位置和尺寸"""',
        "    elements = {}",
        "",
        "    # 根容器 - root_container",
        "    root_style = css_dict['id']['root_container']",
        "    elements['root_container'] = {",
        "        'x': 0,",
        "        'y': 0,",
        "        'width': parse_dimension(root_style.get('width'), SCREEN_WIDTH),",
        "        'height': parse_dimension(root_style.get('height'), SCREEN_HEIGHT),",
        "        'margin': {'top': 0, 'right': 0, 'bottom': 0, 'left': 0},",
        "        'padding': {",
        "            'top': parse_dimension(root_style.get('padding-top', 10)),",
        "            'right': parse_dimension(root_style.get('padding-right', 10)),",
        "            'bottom': parse_dimension(root_style.get('padding-bottom', 10)),",
        "            'left': parse_dimension(root_style.get('padding-left', 10)),",
        "        },",
        "        'border_width': parse_dimension(root_style.get('border-width', 1)),",
        "        'z_index': 0",
        "    }",
        "",
    ]

    # 递归处理元素树，生成布局计算代码
    def process_element(element, parent_name, nest_level):
        if nest_level > MAX_NEST_LEVEL or not element.name:
            return

        # 获取元素ID/类名和样式
        elem_id = element.get("id", "")
        if not elem_id:
            return  # 仅处理有ID的元素

        style = get_element_style(element, css_dict)
        display = style.get("display", "block")

        # 跳过display:none的静态元素
        if display == "none":
            return

        # 生成元素标识
        elem_name = f"'{elem_id}'"
        code_lines.append(f"    # {elem_id} - 层级{nest_level}")
        code_lines.append(f"    elements[{elem_name}] = {{")

        # 处理z-index
        z_index = parse_dimension(style.get("z-index", 0))
        code_lines.append(f"        'z_index': {z_index},")

        # 处理margin (top/right/bottom/left)
        margin = {
            "top": parse_dimension(style.get("margin-top", 0)),
            "right": parse_dimension(style.get("margin-right", 0)),
            "bottom": parse_dimension(style.get("margin-bottom", 0)),
            "left": parse_dimension(style.get("margin-left", 0)),
        }
        code_lines.append(f"        'margin': {{")
        code_lines.append(f"            'top': {margin['top']},")
        code_lines.append(f"            'right': {margin['right']},")
        code_lines.append(f"            'bottom': {margin['bottom']},")
        code_lines.append(f"            'left': {margin['left']},")
        code_lines.append(f"        }},")

        # 处理padding
        padding = {
            "top": parse_dimension(style.get("padding-top", 0)),
            "right": parse_dimension(style.get("padding-right", 0)),
            "bottom": parse_dimension(style.get("padding-bottom", 0)),
            "left": parse_dimension(style.get("padding-left", 0)),
        }
        code_lines.append(f"        'padding': {{")
        code_lines.append(f"            'top': {padding['top']},")
        code_lines.append(f"            'right': {padding['right']},")
        code_lines.append(f"            'bottom': {padding['bottom']},")
        code_lines.append(f"            'left': {padding['left']},")
        code_lines.append(f"        }},")

        # 处理边框宽度
        border_width = parse_dimension(style.get("border-width", 0))
        code_lines.append(f"        'border_width': {border_width},")

        # 处理定位类型
        position = style.get("position", "static")
        code_lines.append(f"        'position': '{position}',")

        parent_elem = elements[parent_name]
        # 处理绝对定位
        if position == "absolute":
            top = parse_dimension(style.get("top", 0))
            left = parse_dimension(style.get("left", 0))
            width = parse_dimension(style.get("width", 0))
            height = parse_dimension(style.get("height", 0))
            code_lines.append(f"        'x': {left},")
            code_lines.append(f"        'y': {top},")
            code_lines.append(f"        'width': {width},")
            code_lines.append(f"        'height': {height},")

        # 处理静态/Flex定位
        else:
            # 基础尺寸
            flex_basis = parse_dimension(style.get("flex-basis", 0))
            parent_width = (
                parent_elem["width"]
                - parent_elem["padding"]["left"]
                - parent_elem["padding"]["right"]
            )
            parent_height = (
                parent_elem["height"]
                - parent_elem["padding"]["top"]
                - parent_elem["padding"]["bottom"]
            )

            width = parse_dimension(style.get("width", flex_basis), parent_width)
            height = parse_dimension(style.get("height", flex_basis), parent_height)

            # 处理flex-grow
            flex_grow = parse_dimension(
                style.get("flex-grow", style.get("flex", 0)), default=0
            )
            code_lines.append(f"        'flex_grow': {flex_grow},")

            # 初始尺寸（后续Flex计算会更新）
            code_lines.append(f"        'width': {width if width > 0 else 0},")
            code_lines.append(f"        'height': {height if height > 0 else 0},")

            # 初始位置（基于父元素和margin/padding）
            base_x = parent_elem["x"] + parent_elem["padding"]["left"] + margin["left"]
            base_y = parent_elem["y"] + parent_elem["padding"]["top"] + margin["top"]
            code_lines.append(f"        'x': {base_x},")
            code_lines.append(f"        'y': {base_y},")

        # 处理文本样式
        font_size = parse_dimension(style.get("font-size", 0))
        line_height = parse_dimension(style.get("line-height", font_size))
        text_align = style.get("text-align", "left")
        code_lines.append(f"        'font_size': {font_size},")
        code_lines.append(f"        'line_height': {line_height},")
        code_lines.append(f"        'text_align': '{text_align}',")

        # 提取{{}}占位符
        text_content = element.text.strip() if element.text else ""
        placeholders = extract_placeholders(text_content)
        if placeholders:
            code_lines.append(f"        'placeholders': {placeholders},")
        else:
            code_lines.append(f"        'text_content': '''{text_content}''',")

        # 标记是否为图片元素
        is_img = element.name == "img"
        code_lines.append(f"        'is_img': {is_img},")

        code_lines.append(f"    }}")
        code_lines.append("")

        # 递归处理子元素
        for child in element.children:
            if child.name and child.name not in ["br", "script"]:
                process_element(child, elem_id, nest_level + 1)

    # 处理根容器的子元素
    root_elem = element_tree.find("div", id="root_container")
    for child in root_elem.children:
        if child.name and child.name not in ["br", "script"]:
            process_element(child, "root_container", 1)

    # Flex布局计算逻辑
    code_lines.extend(
        [
            "    # Flex布局计算 - 处理flex-grow和动态显隐",
            "    # 1. 处理根容器（column方向）",
            "    root_children = [k for k in elements if k != 'root_container' and elements[k]['position'] != 'absolute']",
            "    root_padding = elements['root_container']['padding']",
            "    root_available_height = elements['root_container']['height'] - root_padding['top'] - root_padding['bottom']",
            "    ",
            "    # 计算固定高度元素总占比",
            "    fixed_height_total = 0",
            "    flex_grow_count = 0",
            "    for elem_name in root_children:",
            "        elem = elements[elem_name]",
            "        if elem['position'] == 'absolute':",
            "            continue",
            "        if elem['flex_grow'] == 0:",
            "            fixed_height_total += elem['height'] + elem['margin']['top'] + elem['margin']['bottom']",
            "        else:",
            "            flex_grow_count += 1",
            "    ",
            "    # 计算flex-grow元素的高度",
            "    flex_height = (root_available_height - fixed_height_total) // flex_grow_count if flex_grow_count > 0 else 0",
            "    for elem_name in root_children:",
            "        elem = elements[elem_name]",
            "        if elem['position'] == 'absolute' or elem['flex_grow'] == 0:",
            "            continue",
            "        elem['height'] = flex_height - elem['margin']['top'] - elem['margin']['bottom']",
            "    ",
            "    # 2. 更新元素垂直位置（column方向）",
            "    current_y = elements['root_container']['y'] + root_padding['top']",
            "    for elem_name in ['time_wrap', 'date_wrap', 'divider1', 'lunar_weather_wrap', 'divider2', 'motto_wrap']:",
            "        if elem_name not in elements:",
            "            continue",
            "        elem = elements[elem_name]",
            "        if elem['position'] == 'absolute':",
            "            continue",
            "        # 更新Y坐标",
            "        elem['y'] = current_y + elem['margin']['top']",
            "        # 更新X坐标（水平居中）",
            "        if elem.get('text_align') == 'center' and elem['width'] == 0:",
            "            elem['x'] = elements['root_container']['x'] + (elements['root_container']['width'] - elem['width']) // 2",
            "        # 移动到下一个元素位置",
            "        current_y = elem['y'] + elem['height'] + elem['margin']['bottom']",
            "    ",
            "    # 3. 处理lunar_weather_wrap的水平Flex布局",
            "    if 'lunar_weather_wrap' in elements:",
            "        weather_wrap = elements['lunar_weather_wrap']",
            "        # 平分宽度给lunar_wrap和weather_wrap",
            "        child_width = (weather_wrap['width'] - weather_wrap['padding']['left'] - weather_wrap['padding']['right']) // 2",
            "        if 'lunar_wrap' in elements:",
            "            elements['lunar_wrap']['width'] = child_width - elements['lunar_wrap']['margin']['left'] - elements['lunar_wrap']['margin']['right']",
            "            elements['lunar_wrap']['x'] = weather_wrap['x'] + weather_wrap['padding']['left']",
            "        if 'weather_wrap' in elements:",
            "            elements['weather_wrap']['width'] = child_width - elements['weather_wrap']['margin']['left'] - elements['weather_wrap']['margin']['right']",
            "            elements['weather_wrap']['x'] = weather_wrap['x'] + weather_wrap['padding']['left'] + child_width",
            "    ",
            "    return elements",
            "",
        ]
    )

    # 绘制逻辑
    code_lines.extend(
        [
            "# 绘制元素",
            "def draw_elements(draw, elements):",
            '    """绘制所有元素（按z-index排序）"""',
            "    # 按z-index排序，高的后绘制",
            "    sorted_elems = sorted([(k, v) for k, v in elements.items()], key=lambda x: x[1]['z_index'])",
            "    ",
            "    # 1. 绘制根容器边框",
            "    root = elements['root_container']",
            "    draw.rectangle([root['x'], root['y'], root['x'] + root['width'], root['y'] + root['height']], outline='black', width=root['border_width'])",
            "    ",
            "    for elem_name, elem in sorted_elems:",
            "        if elem_name == 'root_container':",
            "            continue",
            "        ",
            "        x1 = elem['x']",
            "        y1 = elem['y']",
            "        x2 = elem['x'] + elem['width']",
            "        y2 = elem['y'] + elem['height']",
            "        ",
            "        # 绘制边框（如果有）",
            "        if elem['border_width'] > 0:",
            "            draw.rectangle([x1, y1, x2, y2], outline='black', width=elem['border_width'])",
            "        ",
            "        # 绘制分割线",
            "        if 'divider' in elem_name:",
            "            draw.rectangle([x1, y1, x2, y2], fill='black')",
            "        ",
            "        # 绘制文本元素",
            "        if elem['font_size'] > 0 and not elem['is_img']:",
            "            font = load_font(elem['font_size'])",
            "            # 获取文本内容（替换占位符）",
            "            if 'placeholders' in elem:",
            "                text = ''",
            "                for ph in elem['placeholders']:",
            "                    # 处理嵌套占位符",
            "                    if ph in mock_data:",
            "                        text = mock_data[ph]",
            "                    elif 'day' in ph and 'weather_3days' in mock_data:",
            "                        idx = int(ph.replace('day', '')) - 1",
            "                        if idx < len(mock_data['weather_3days']):",
            "                            text = mock_data['weather_3days'][idx].get('day', '')",
            "                    elif 'desc' in ph and 'weather_3days' in mock_data:",
            "                        idx = int(ph.replace('desc', '')) - 1",
            "                        if idx < len(mock_data['weather_3days']):",
            "                            text = mock_data['weather_3days'][idx].get('desc', '')",
            "            else:",
            "                text = elem.get('text_content', '')",
            "                # 特殊处理时间文本",
            "                if elem_name == 'time_wrap':",
            "                    text = mock_data.get('time', '00:00')",
            "                # 特殊处理格言文本",
            "                elif elem_name == 'motto_content':",
            "                    text = mock_data.get('motto_content', '')",
            "                elif elem_name == 'motto_source':",
            "                    text = mock_data.get('motto_source', '')",
            "            ",
            "            if not text:",
            "                continue",
            "            ",
            "            # 计算文本布局",
            "            max_text_width = elem['width'] - elem['padding']['left'] - elem['padding']['right']",
            "            lines, text_height = calculate_text_layout(draw, text, font, max_text_width, elem['line_height'])",
            "            ",
            "            # 文本垂直居中基准",
            "            text_y_base = y1 + elem['padding']['top'] + (elem['height'] - text_height) // 2",
            "            ",
            "            # 处理文本对齐",
            "            for i, line in enumerate(lines):",
            "                line_y = text_y_base + i * elem['line_height']",
            "                if elem['text_align'] == 'center':",
            "                    line_width = draw.textlength(line, font=font)",
            "                    line_x = x1 + (elem['width'] - line_width) // 2",
            "                elif elem['text_align'] == 'right':",
            "                    line_width = draw.textlength(line, font=font)",
            "                    line_x = x2 - elem['padding']['right'] - line_width",
            "                else:",
            "                    line_x = x1 + elem['padding']['left']",
            "                ",
            "                # 绘制单行文本",
            "                draw.text((line_x, line_y), line, fill='black', font=font)",
            "        ",
            "        # 绘制图片元素（占位框）",
            "        if elem['is_img']:",
            "            # 绘制图片占位框",
            "            draw.rectangle([x1, y1, x2, y2], outline='blue', width=2)",
            "            # 标注图片信息",
            "            font = load_font(10)",
            "            draw.text((x1 + 2, y1 + 2), elem_name, fill='blue', font=font)",
            "            # 特殊处理充电图标显隐",
            "            if elem_name == 'charging_icon' and not mock_data.get('charging', False):",
            "                draw.rectangle([x1, y1, x2, y2], fill='white')  # 覆盖隐藏",
            "",
            "    return draw",
            "",
        ]
    )

    # 主函数
    code_lines.extend(
        [
            "# 主渲染函数",
            "def render_layout(output_path='info_panel.png'):",
            '    """渲染布局并保存为PNG"""',
            "    # 初始化画布",
            "    img, draw = init_canvas()",
            "    ",
            "    # 计算布局",
            "    elements = calculate_layout(draw)",
            "    ",
            "    # 绘制元素",
            "    draw = draw_elements(draw, elements)",
            "    ",
            "    # 保存图片",
            "    img.save(output_path)",
            '    print(f"信息面板已渲染到: {output_path}")',
            "    return img",
            "",
            "# 执行渲染",
            "if __name__ == '__main__':",
            "    render_layout()",
        ]
    )

    return "\n".join(code_lines)


def extract_font_path(css_text):
    """从CSS中提取字体文件路径"""
    font_path_match = re.search(
        r'src:\s*url\(["\'](.*?)["\']\)', css_text, re.IGNORECASE
    )
    if font_path_match:
        return font_path_match.group(1)
    return "assets/fonts/MapleMono-NF-CN-Regular.ttf"


def main():
    # 解析命令行参数
    parser = argparse.ArgumentParser(
        description="解析800x480信息面板HTML，生成PIL渲染代码"
    )
    parser.add_argument("--input", default="input.html", help="输入HTML文件路径")
    parser.add_argument("--output", default="output.py", help="输出Python渲染代码路径")
    args = parser.parse_args()

    # 读取HTML文件
    try:
        with open(args.input, "r", encoding="utf-8") as f:
            html_content = f.read()
    except FileNotFoundError:
        print(f"错误：找不到输入文件 {args.input}")
        return

    # 解析HTML和CSS
    soup = BeautifulSoup(html_content, "html.parser")
    css_text = extract_css_from_html(html_content)
    css_dict = parse_css(css_text)

    # 验证根容器
    root_container = soup.find("div", id="root_container")
    if not root_container:
        print('错误：HTML文件必须包含<div id="root_container">作为根容器')
        return

    # 提取字体路径
    font_path = extract_font_path(css_text)

    # 生成Python渲染代码
    python_code = generate_python_code(soup, css_dict, font_path)

    # 保存生成的代码
    with open(args.output, "w", encoding="utf-8") as f:
        f.write(python_code)

    print(f"成功生成渲染代码：{args.output}")
    print("使用说明：")
    print(f"1. 安装依赖：pip install pillow beautifulsoup4 cssutils")
    print(f"2. 运行生成的代码：python {args.output}")
    print(f"3. 修改mock_data字典可替换动态占位符内容")


if __name__ == "__main__":
    main()
