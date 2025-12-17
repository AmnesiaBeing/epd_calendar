import json
import re
import os
from lxml import html

# 配置项
HTML_PATH = "test.html"
OUTPUT_PARAMS_PATH = "runtime_params_complex.json"
ROOT_WIDTH = 800
ROOT_HEIGHT = 480

# 支持的属性（包含padding/margin全方向）
SUPPORTED_PROPERTIES = [
    "width",
    "height",
    "position",
    "left",
    "top",
    "right",
    "bottom",
    "display",
    "flex-direction",
    "justify-content",
    "align-items",
    "flex-grow",
    "flex-basis",
    "margin-left",
    "margin-top",
    "margin-bottom",
    "margin-right",
    "padding-left",
    "padding-top",
    "padding-bottom",
    "padding-right",
    "border-width",
    "border-style",
    "font-size",
    "line-height",
    "text-align",
    "white-space",
    "word-break",
]


def parse_css_custom(style_text):
    """自定义CSS解析器（修复：支持复合padding/margin拆分）"""
    css_rules = {}
    # 清理注释和空白
    style_text = re.sub(r"/\*[\s\S]*?\*/", "", style_text)
    style_text = re.sub(r"\s+", " ", style_text)
    style_text = re.sub(r"\{ ", "{", style_text)
    style_text = re.sub(r" \}", "}", style_text)

    # 匹配规则：.class { prop: value; ... }
    rule_pattern = re.compile(r"(\.[a-zA-Z0-9_-]+)\s*\{\s*(.*?)\s*\}")
    matches = rule_pattern.findall(style_text)

    for selector, props_text in matches:
        selector = selector.strip()
        props = {}
        # 拆分属性
        prop_pattern = re.compile(r"([a-zA-Z0-9_-]+)\s*:\s*([^;]+)\s*;")
        prop_matches = prop_pattern.findall(props_text)

        for prop_name, prop_value in prop_matches:
            prop_name = prop_name.strip()
            prop_value = prop_value.strip()

            # 处理复合padding/margin（如padding:10px → 四个方向都是10px）
            if prop_name == "padding" and prop_value.endswith("px"):
                val = prop_value.replace("px", "")
                props["padding-left"] = val + "px"
                props["padding-top"] = val + "px"
                props["padding-right"] = val + "px"
                props["padding-bottom"] = val + "px"
            elif prop_name == "margin" and prop_value.endswith("px"):
                val = prop_value.replace("px", "")
                props["margin-left"] = val + "px"
                props["margin-top"] = val + "px"
                props["margin-right"] = val + "px"
                props["margin-bottom"] = val + "px"
            elif prop_name in SUPPORTED_PROPERTIES:
                props[prop_name] = prop_value

        css_rules[selector] = props

    return css_rules


def parse_size(size_str, parent_size_val):
    """独立解析尺寸（px/百分比）"""
    if not size_str or "{" in size_str:  # 跳过占位符
        return 0.0
    if "px" in size_str:
        return float(size_str.replace("px", ""))
    elif "%" in size_str:
        return parent_size_val * float(size_str.replace("%", "")) / 100
    else:
        return float(size_str) if size_str.replace(".", "").isdigit() else 0.0


def calculate_flex_child_pos(
    parent_pos, parent_size, child_idx, child_count, child_size, flex_justify
):
    """计算Flex子元素的位置（仅支持row方向 + space-around/space-between/center）"""
    # 计算子元素总宽度
    total_child_width = child_size["width"] * child_count
    # 剩余空间
    remaining_space = parent_size["width"] - total_child_width
    x = parent_pos["x"]
    y = parent_pos["y"] + (parent_size["height"] - child_size["height"]) / 2  # 垂直居中

    if flex_justify == "space-around":
        # space-around：两端留白 = 剩余空间/(child_count+1)
        gap = remaining_space / (child_count + 1)
        x += gap * (child_idx + 1)
    elif flex_justify == "space-between":
        # space-between：两端无留白，中间均分
        if child_count == 1:
            x += (parent_size["width"] - child_size["width"]) / 2
        else:
            gap = remaining_space / (child_count - 1)
            x += gap * child_idx
    elif flex_justify == "center":
        # center：整体居中
        x += (parent_size["width"] - total_child_width) / 2 + child_size[
            "width"
        ] * child_idx
    else:  # flex-start
        x += child_size["width"] * child_idx

    return {"x": x, "y": y}


def calculate_absolute_pos(elem_style, parent_pos, parent_size):
    """修复：正确计算绝对坐标（累加padding/margin）"""
    # 提取所有偏移量（默认0）
    margin_left = parse_size(elem_style.get("margin-left", "0px"), parent_size["width"])
    margin_top = parse_size(elem_style.get("margin-top", "0px"), parent_size["height"])
    padding_left = parse_size(
        elem_style.get("padding-left", "0px"), parent_size["width"]
    )
    padding_top = parse_size(
        elem_style.get("padding-top", "0px"), parent_size["height"]
    )

    # 解析尺寸
    width = parse_size(elem_style.get("width", "0px"), parent_size["width"])
    height = parse_size(elem_style.get("height", "0px"), parent_size["height"])

    # 绝对坐标 = 父坐标 + margin + padding
    x = parent_pos["x"] + margin_left + padding_left
    y = parent_pos["y"] + margin_top + padding_top

    return {"x": x, "y": y, "width": width, "height": height}


def extract_dynamic_placeholders(text):
    """提取动态占位符"""
    if not text:
        return []
    return re.findall(r"\{(\w+)\}", text)


def get_elem_style(elem_class, css_rules):
    """根据类名获取样式"""
    return css_rules.get(f".{elem_class}", {})


def parse_html():
    """修复：正确计算多层嵌套坐标"""
    # 1. 解析HTML
    tree = html.parse(HTML_PATH)
    root_elem = tree.xpath('//div[@class="page-container"]')[0]

    # 2. 解析CSS
    style_elem = tree.xpath("//style")[0]
    style_text = style_elem.text or ""
    css_rules = parse_css_custom(style_text)

    # 3. 根容器基础信息
    root_pos = {"x": 0.0, "y": 0.0}
    root_size = {"width": ROOT_WIDTH, "height": ROOT_HEIGHT}

    # --------------------------
    # 逐层计算父容器坐标（核心修复）
    # --------------------------
    # 第2层：flex-root-container
    flex_root_style = get_elem_style("flex-root-container", css_rules)
    flex_root_pos = calculate_absolute_pos(flex_root_style, root_pos, root_size)
    flex_root_size = {
        "width": ROOT_WIDTH
        - parse_size(flex_root_style.get("padding-left", "0px"), ROOT_WIDTH)
        - parse_size(flex_root_style.get("padding-right", "0px"), ROOT_WIDTH),
        "height": ROOT_HEIGHT
        - parse_size(flex_root_style.get("padding-top", "0px"), ROOT_HEIGHT)
        - parse_size(flex_root_style.get("padding-bottom", "0px"), ROOT_HEIGHT),
    }

    # 第3层：flex-row-top
    flex_row_top_style = get_elem_style("flex-row-top", css_rules)
    flex_row_top_pos = calculate_absolute_pos(
        flex_row_top_style, flex_root_pos, flex_root_size
    )
    flex_row_top_size = {
        "width": flex_root_size["width"],
        "height": parse_size(
            flex_row_top_style.get("height", "150px"), flex_root_size["height"]
        ),
    }
    flex_row_top_justify = flex_row_top_style.get("justify-content", "flex-start")

    # 第4层：static-item-group-1/2/3（flex-row-top的子元素，共3个）
    group_count = 3
    # group1
    static_group_1_style = get_elem_style("static-item-group-1", css_rules)
    static_group_1_size = {
        "width": parse_size(
            static_group_1_style.get("width", "200px"), flex_row_top_size["width"]
        ),
        "height": parse_size(
            static_group_1_style.get("height", "100%"), flex_row_top_size["height"]
        ),
    }
    static_group_1_pos = calculate_flex_child_pos(
        flex_row_top_pos,
        flex_row_top_size,
        0,
        group_count,
        static_group_1_size,
        flex_row_top_justify,
    )

    # group2
    static_group_2_style = get_elem_style("static-item-group-2", css_rules)
    static_group_2_size = {
        "width": parse_size(
            static_group_2_style.get("width", "180px"), flex_row_top_size["width"]
        ),
        "height": parse_size(
            static_group_2_style.get("height", "100%"), flex_row_top_size["height"]
        ),
    }
    static_group_2_pos = calculate_flex_child_pos(
        flex_row_top_pos,
        flex_row_top_size,
        1,
        group_count,
        static_group_2_size,
        flex_row_top_justify,
    )

    # group3
    static_group_3_style = get_elem_style("static-item-group-3", css_rules)
    static_group_3_size = {
        "width": parse_size(
            static_group_3_style.get("width", "180px"), flex_row_top_size["width"]
        ),
        "height": parse_size(
            static_group_3_style.get("height", "100%"), flex_row_top_size["height"]
        ),
    }
    static_group_3_pos = calculate_flex_child_pos(
        flex_row_top_pos,
        flex_row_top_size,
        2,
        group_count,
        static_group_3_size,
        flex_row_top_justify,
    )

    # 第3层：flex-row-middle
    flex_row_middle_style = get_elem_style("flex-row-middle", css_rules)
    flex_row_middle_pos = {
        "x": flex_root_pos["x"],
        "y": flex_row_top_pos["y"]
        + flex_row_top_size["height"]
        + parse_size(
            flex_row_top_style.get("margin-bottom", "10px"), flex_root_size["height"]
        ),
    }
    flex_row_middle_size = {
        "width": flex_root_size["width"],
        "height": parse_size(
            flex_row_middle_style.get("height", "200px"), flex_root_size["height"]
        ),
    }

    # 第4层：dynamic-item-group-left/right
    dynamic_group_left_style = get_elem_style("dynamic-item-group-left", css_rules)
    dynamic_group_left_size = {
        "width": parse_size(
            dynamic_group_left_style.get("width", "350px"),
            flex_row_middle_size["width"],
        ),
        "height": flex_row_middle_size["height"],
    }
    dynamic_group_left_pos = {
        "x": flex_row_middle_pos["x"]
        + parse_size(
            flex_row_middle_style.get("margin-left", "0px"),
            flex_row_middle_size["width"],
        ),
        "y": flex_row_middle_pos["y"],
    }

    dynamic_group_right_style = get_elem_style("dynamic-item-group-right", css_rules)
    dynamic_group_right_size = {
        "width": parse_size(
            dynamic_group_right_style.get("width", "350px"),
            flex_row_middle_size["width"],
        ),
        "height": flex_row_middle_size["height"],
    }
    dynamic_group_right_pos = {
        "x": flex_row_middle_pos["x"]
        + flex_row_middle_size["width"]
        - dynamic_group_right_size["width"],
        "y": flex_row_middle_pos["y"],
    }

    # 第3层：flex-row-bottom
    flex_row_bottom_style = get_elem_style("flex-row-bottom", css_rules)
    flex_row_bottom_pos = {
        "x": flex_root_pos["x"],
        "y": flex_row_middle_pos["y"]
        + flex_row_middle_size["height"]
        + parse_size(
            flex_row_middle_style.get("margin-bottom", "10px"), flex_root_size["height"]
        ),
    }
    flex_row_bottom_size = {
        "width": flex_root_size["width"],
        "height": parse_size(
            flex_row_bottom_style.get("height", "80px"), flex_root_size["height"]
        ),
    }

    # --------------------------
    # 提取静态元素（带正确坐标）
    # --------------------------
    static_elements = []

    # 1. 静态空白容器（static-empty-box）
    empty_box_elem = tree.xpath('//div[@class="static-empty-box"]')[0]
    empty_box_style = get_elem_style("static-empty-box", css_rules)
    empty_box_pos = calculate_absolute_pos(
        empty_box_style, static_group_1_pos, static_group_1_size
    )
    static_elements.append(
        {
            "type": "empty_box",
            "name": "static_empty_box",
            "class": "static-empty-box",
            "pos": empty_box_pos,
            "style": {
                "border_width": parse_size(
                    empty_box_style.get("border-width", "0px"), empty_box_pos["width"]
                ),
                "border_style": empty_box_style.get("border-style", "none"),
                "display": empty_box_style.get("display", "block"),
            },
        }
    )

    # 2. 静态文字1（static-text-1）
    static_text_1_elem = tree.xpath('//span[@class="static-text-1"]')[0]
    static_text_1_style = get_elem_style("static-text-1", css_rules)
    static_text_1_pos = calculate_absolute_pos(
        static_text_1_style,
        {
            "x": static_group_1_pos["x"],
            "y": empty_box_pos["y"] + empty_box_pos["height"] + 5,
        },  # 5px margin-bottom
        static_group_1_size,
    )
    static_elements.append(
        {
            "type": "text",
            "name": "static_text_1",
            "class": "static-text-1",
            "pos": static_text_1_pos,
            "content": (
                static_text_1_elem.text.strip() if static_text_1_elem.text else ""
            ),
            "style": {
                "font_size": parse_size(
                    static_text_1_style.get("font-size", "14px"),
                    static_text_1_pos["width"],
                ),
                "line_height": parse_size(
                    static_text_1_style.get("line-height", "18px"),
                    static_text_1_pos["height"],
                ),
                "text_align": static_text_1_style.get("text-align", "left"),
                "display": static_text_1_style.get("display", "block"),
            },
            "placeholders": [],
        }
    )

    # 3. 静态图片1（static-img-1）
    static_img_1_elem = tree.xpath('//img[@class="static-img-1"]')[0]
    static_img_1_wrapper_style = get_elem_style("static-img-1-wrapper", css_rules)
    static_img_1_wrapper_pos = calculate_absolute_pos(
        static_img_1_wrapper_style, static_group_2_pos, static_group_2_size
    )
    static_img_1_style = get_elem_style("static-img-1", css_rules)
    static_img_1_pos = calculate_absolute_pos(
        static_img_1_style, static_img_1_wrapper_pos, static_group_2_size
    )
    static_elements.append(
        {
            "type": "img",
            "name": "static_img_1",
            "class": "static-img-1",
            "pos": static_img_1_pos,
            "src": static_img_1_elem.get("src", ""),
            "style": {
                "border_width": parse_size(
                    static_img_1_style.get("border-width", "0px"),
                    static_img_1_pos["width"],
                ),
                "border_style": static_img_1_style.get("border-style", "none"),
                "display": static_img_1_wrapper_style.get("display", "block"),
            },
            "src_placeholders": [],
            "display_placeholders": [],
        }
    )

    # 4. 静态图片2（static-img-2）
    static_img_2_elem = tree.xpath('//img[@class="static-img-2"]')[0]
    static_img_2_wrapper_style = get_elem_style("static-img-2-wrapper", css_rules)
    static_img_2_wrapper_pos = calculate_absolute_pos(
        static_img_2_wrapper_style,
        {
            "x": static_group_2_pos["x"],
            "y": static_img_1_wrapper_pos["y"] + static_img_1_wrapper_pos["height"],
        },
        static_group_2_size,
    )
    static_img_2_style = get_elem_style("static-img-2", css_rules)
    static_img_2_pos = calculate_absolute_pos(
        static_img_2_style, static_img_2_wrapper_pos, static_group_2_size
    )
    static_elements.append(
        {
            "type": "img",
            "name": "static_img_2",
            "class": "static-img-2",
            "pos": static_img_2_pos,
            "src": static_img_2_elem.get("src", ""),
            "style": {
                "border_width": parse_size(
                    static_img_2_style.get("border-width", "0px"),
                    static_img_2_pos["width"],
                ),
                "border_style": static_img_2_style.get("border-style", "none"),
                "display": static_img_2_wrapper_style.get("display", "block"),
            },
            "src_placeholders": [],
            "display_placeholders": [],
        }
    )

    # 5. 静态图片3（static-img-3）
    static_img_3_elem = tree.xpath('//img[@class="static-img-3"]')[0]
    static_img_3_wrapper_style = get_elem_style("static-img-3-wrapper", css_rules)
    static_img_3_wrapper_pos = calculate_absolute_pos(
        static_img_3_wrapper_style, static_group_3_pos, static_group_3_size
    )
    static_img_3_style = get_elem_style("static-img-3", css_rules)
    static_img_3_pos = calculate_absolute_pos(
        static_img_3_style, static_img_3_wrapper_pos, static_group_3_size
    )
    static_elements.append(
        {
            "type": "img",
            "name": "static_img_3",
            "class": "static-img-3",
            "pos": static_img_3_pos,
            "src": static_img_3_elem.get("src", ""),
            "style": {
                "border_width": parse_size(
                    static_img_3_style.get("border-width", "0px"),
                    static_img_3_pos["width"],
                ),
                "border_style": static_img_3_style.get("border-style", "none"),
                "display": static_img_3_wrapper_style.get("display", "block"),
            },
            "src_placeholders": [],
            "display_placeholders": [],
        }
    )

    # 6. 静态文字2（static-text-2）
    static_text_2_elem = tree.xpath('//span[@class="static-text-2"]')[0]
    static_text_2_style = get_elem_style("static-text-2", css_rules)
    static_text_2_pos = calculate_absolute_pos(
        static_text_2_style,
        {
            "x": static_group_3_pos["x"],
            "y": static_img_3_wrapper_pos["y"] + static_img_3_wrapper_pos["height"],
        },
        static_group_3_size,
    )
    static_elements.append(
        {
            "type": "text",
            "name": "static_text_2",
            "class": "static-text-2",
            "pos": static_text_2_pos,
            "content": (
                static_text_2_elem.text.strip() if static_text_2_elem.text else ""
            ),
            "style": {
                "font_size": parse_size(
                    static_text_2_style.get("font-size", "14px"),
                    static_text_2_pos["width"],
                ),
                "line_height": parse_size(
                    static_text_2_style.get("line-height", "18px"),
                    static_text_2_pos["height"],
                ),
                "text_align": static_text_2_style.get("text-align", "left"),
                "display": static_text_2_style.get("display", "block"),
            },
            "placeholders": [],
        }
    )

    # 7. 底部静态文字（static-text-bottom）
    static_text_bottom_elem = tree.xpath('//span[@class="static-text-bottom"]')[0]
    static_text_bottom_wrapper_style = get_elem_style(
        "static-text-bottom-wrapper", css_rules
    )
    static_text_bottom_wrapper_pos = calculate_absolute_pos(
        static_text_bottom_wrapper_style, flex_row_bottom_pos, flex_row_bottom_size
    )
    static_text_bottom_style = get_elem_style("static-text-bottom", css_rules)
    static_text_bottom_pos = calculate_absolute_pos(
        static_text_bottom_style,
        static_text_bottom_wrapper_pos,
        {
            "width": parse_size(
                static_text_bottom_wrapper_style.get("width", "700px"),
                flex_row_bottom_size["width"],
            ),
            "height": flex_row_bottom_size["height"],
        },
    )
    static_elements.append(
        {
            "type": "text",
            "name": "static_text_bottom",
            "class": "static-text-bottom",
            "pos": static_text_bottom_pos,
            "content": (
                static_text_bottom_elem.text.strip()
                if static_text_bottom_elem.text
                else ""
            ),
            "style": {
                "font_size": parse_size(
                    static_text_bottom_style.get("font-size", "18px"),
                    static_text_bottom_pos["width"],
                ),
                "line_height": parse_size(
                    static_text_bottom_style.get("line-height", "22px"),
                    static_text_bottom_pos["height"],
                ),
                "text_align": static_text_bottom_style.get("text-align", "center"),
                "display": static_text_bottom_style.get("display", "block"),
            },
            "placeholders": [],
        }
    )

    # --------------------------
    # 提取动态元素（带正确坐标）
    # --------------------------
    dynamic_elements = []

    # 1. 动态文字（dynamic-text）
    dynamic_text_elem = tree.xpath('//span[@class="dynamic-text"]')[0]
    dynamic_text_wrapper_style = get_elem_style("dynamic-text-wrapper", css_rules)
    dynamic_text_wrapper_pos = calculate_absolute_pos(
        dynamic_text_wrapper_style, dynamic_group_left_pos, dynamic_group_left_size
    )
    dynamic_text_style = get_elem_style("dynamic-text", css_rules)
    dynamic_text_pos = calculate_absolute_pos(
        dynamic_text_style, dynamic_text_wrapper_pos, dynamic_group_left_size
    )
    dynamic_elements.append(
        {
            "type": "text",
            "name": "dynamic_text",
            "class": "dynamic-text",
            "pos": dynamic_text_pos,
            "content": dynamic_text_elem.text.strip() if dynamic_text_elem.text else "",
            "style": {
                "font_size": parse_size(
                    dynamic_text_style.get("font-size", "16px"),
                    dynamic_text_pos["width"],
                ),
                "line_height": parse_size(
                    dynamic_text_style.get("line-height", "20px"),
                    dynamic_text_pos["height"],
                ),
                "text_align": dynamic_text_style.get("text-align", "left"),
                "display": dynamic_text_style.get("display", "block"),
            },
            "placeholders": extract_dynamic_placeholders(dynamic_text_elem.text or ""),
            "default_content": "默认动态文字：超长内容测试break-all换行，嵌套层级超过5层！",
        }
    )

    # 2. 动态图片1（dynamic-img-1）
    dynamic_img_1_elem = tree.xpath('//img[@class="dynamic-img-1"]')[0]
    dynamic_img_1_wrapper_style = get_elem_style("dynamic-img-1-wrapper", css_rules)
    dynamic_img_1_wrapper_pos = calculate_absolute_pos(
        dynamic_img_1_wrapper_style,
        {
            "x": dynamic_group_left_pos["x"],
            "y": dynamic_text_wrapper_pos["y"] + dynamic_text_wrapper_pos["height"],
        },
        dynamic_group_left_size,
    )
    dynamic_img_1_style = get_elem_style("dynamic-img-1", css_rules)
    dynamic_img_1_pos = calculate_absolute_pos(
        dynamic_img_1_style, dynamic_img_1_wrapper_pos, dynamic_group_left_size
    )
    dynamic_elements.append(
        {
            "type": "img",
            "name": "dynamic_img_1",
            "class": "dynamic-img-1",
            "pos": dynamic_img_1_pos,
            "src": dynamic_img_1_elem.get("src", ""),
            "style": {
                "border_width": parse_size(
                    dynamic_img_1_style.get("border-width", "1px"),
                    dynamic_img_1_pos["width"],
                ),
                "border_style": dynamic_img_1_style.get("border-style", "solid"),
                "display": dynamic_img_1_wrapper_style.get("display", "block"),
            },
            "src_placeholders": extract_dynamic_placeholders(
                dynamic_img_1_elem.get("src", "")
            ),
            "display_placeholders": extract_dynamic_placeholders(
                dynamic_img_1_wrapper_style.get("display", "")
            ),
            "default_src": "./dynamic_img_1.bmp",
            "default_display": "block",
        }
    )

    # 3. 动态图片2（dynamic-img-2）
    dynamic_img_2_elem = tree.xpath('//img[@class="dynamic-img-2"]')[0]
    dynamic_img_2_wrapper_style = get_elem_style("dynamic-img-2-wrapper", css_rules)
    dynamic_img_2_wrapper_pos = calculate_absolute_pos(
        dynamic_img_2_wrapper_style, dynamic_group_right_pos, dynamic_group_right_size
    )
    dynamic_img_2_style = get_elem_style("dynamic-img-2", css_rules)
    dynamic_img_2_pos = calculate_absolute_pos(
        dynamic_img_2_style, dynamic_img_2_wrapper_pos, dynamic_group_right_size
    )
    dynamic_elements.append(
        {
            "type": "img",
            "name": "dynamic_img_2",
            "class": "dynamic-img-2",
            "pos": dynamic_img_2_pos,
            "src": dynamic_img_2_elem.get("src", ""),
            "style": {
                "border_width": parse_size(
                    dynamic_img_2_style.get("border-width", "1px"),
                    dynamic_img_2_pos["width"],
                ),
                "border_style": dynamic_img_2_style.get("border-style", "solid"),
                "display": dynamic_img_2_wrapper_style.get("display", "block"),
            },
            "src_placeholders": extract_dynamic_placeholders(
                dynamic_img_2_elem.get("src", "")
            ),
            "display_placeholders": extract_dynamic_placeholders(
                dynamic_img_2_wrapper_style.get("display", "")
            ),
            "default_src": "./dynamic_img_2.bmp",
            "default_display": "block",
        }
    )

    # 4. 动态图片3（dynamic-img-3）
    dynamic_img_3_elem = tree.xpath('//img[@class="dynamic-img-3"]')[0]
    dynamic_img_3_wrapper_style = get_elem_style("dynamic-img-3-wrapper", css_rules)
    dynamic_img_3_wrapper_pos = calculate_absolute_pos(
        dynamic_img_3_wrapper_style,
        {
            "x": dynamic_group_right_pos["x"],
            "y": dynamic_img_2_wrapper_pos["y"] + dynamic_img_2_wrapper_pos["height"],
        },
        dynamic_group_right_size,
    )
    dynamic_img_3_style = get_elem_style("dynamic-img-3", css_rules)
    dynamic_img_3_pos = calculate_absolute_pos(
        dynamic_img_3_style, dynamic_img_3_wrapper_pos, dynamic_group_right_size
    )
    dynamic_elements.append(
        {
            "type": "img",
            "name": "dynamic_img_3",
            "class": "dynamic-img-3",
            "pos": dynamic_img_3_pos,
            "src": dynamic_img_3_elem.get("src", ""),
            "style": {
                "border_width": parse_size(
                    dynamic_img_3_style.get("border-width", "1px"),
                    dynamic_img_3_pos["width"],
                ),
                "border_style": dynamic_img_3_style.get("border-style", "solid"),
                "display": dynamic_img_3_wrapper_style.get("display", "block"),
            },
            "src_placeholders": extract_dynamic_placeholders(
                dynamic_img_3_elem.get("src", "")
            ),
            "display_placeholders": extract_dynamic_placeholders(
                dynamic_img_3_wrapper_style.get("display", "")
            ),
            "default_src": "./dynamic_img_3.bmp",
            "default_display": "block",
        }
    )

    # --------------------------
    # 整合参数并保存
    # --------------------------
    runtime_params = {
        "root_size": {"width": ROOT_WIDTH, "height": ROOT_HEIGHT},
        "static_elements": static_elements,
        "dynamic_elements": dynamic_elements,
        "dynamic_placeholders": {
            "display": ["dynamic_display_1", "dynamic_display_2", "dynamic_display_3"],
            "src": ["dynamic_img_1_src", "dynamic_img_2_src", "dynamic_img_3_src"],
            "text": ["dynamic_text"],
        },
    }

    with open(OUTPUT_PARAMS_PATH, "w", encoding="utf-8") as f:
        json.dump(runtime_params, f, ensure_ascii=False, indent=2)

    print(f"✅ 解析完成！参数已保存至 {OUTPUT_PARAMS_PATH}")
    print(
        f"📌 解析结果：静态元素{len(static_elements)}个，动态元素{len(dynamic_elements)}个"
    )
    # 打印关键坐标验证
    print(f"🔍 关键坐标验证：")
    print(f"   - 根容器：{root_pos} (尺寸：{root_size})")
    print(f"   - 顶部flex行：{flex_row_top_pos} (尺寸：{flex_row_top_size})")
    print(f"   - 静态空白容器：{static_elements[0]['pos']}")
    print(f"   - 动态文字：{dynamic_elements[0]['pos']}")


if __name__ == "__main__":
    parse_html()
