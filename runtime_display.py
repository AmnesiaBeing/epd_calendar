import json
import os
from PIL import Image, ImageDraw, ImageFont
from svglib.svglib import svg2rlg
from reportlab.graphics import renderPM
import io

# 配置项
RUNTIME_PARAMS_PATH = "runtime_params_complex.json"
# 字体路径（替换为本地有效路径）
FONT_PATH = "assets/fonts/MapleMono-NF-CN-Regular.ttf"

# 动态参数（模拟嵌入式传入）
DYNAMIC_PARAMS = {
    "dynamic_text": "动态更新文字：复杂嵌套测试，这是超长的文字内容，需要自动换行，break-all规则下任意位置断行，测试多层flex布局的文字图片混排效果！",
    "dynamic_img_1_src": "assets/icons/time_digit/digit_5.svg",
    "dynamic_img_2_src": "assets/icons/time_digit/digit_6.svg",
    "dynamic_img_3_src": "assets/icons/time_digit/digit_7.svg",
    "dynamic_display_1": "block",
    "dynamic_display_2": "block",
    "dynamic_display_3": "block",
}


def draw_text_with_wrap(
    draw, text, pos, font_size, line_height, max_width, fill="black"
):
    """绘制自动换行文字（break-all）"""
    try:
        font = ImageFont.truetype(FONT_PATH, font_size)
    except:
        font = ImageFont.load_default()
        print(f"警告：字体文件 {FONT_PATH} 不存在，使用默认字体")

    lines = []
    current_line = ""
    for char in text:
        current_width = draw.textlength(current_line + char, font=font)
        if current_width > max_width:
            lines.append(current_line)
            current_line = char
        else:
            current_line += char
    if current_line:
        lines.append(current_line)

    y = pos["y"]
    for line in lines:
        draw.text((pos["x"], y), line, fill=fill, font=font)
        y += line_height


def draw_border(draw, pos, border_width, fill="black"):
    """绘制边框"""
    x, y, w, h = pos["x"], pos["y"], pos["width"], pos["height"]
    if border_width <= 0:
        return
    # 上
    draw.rectangle([x, y, x + w, y + border_width], fill=fill)
    # 下
    draw.rectangle([x, y + h - border_width, x + w, y + h], fill=fill)
    # 左
    draw.rectangle([x, y, x + border_width, y + h], fill=fill)
    # 右
    draw.rectangle([x + w - border_width, y, x + w, y + h], fill=fill)


def svg_to_pil_image(svg_path, width=None, height=None):
    """
    将SVG转换为PIL Image对象（位图）
    :param svg_path: SVG文件路径
    :param width: 目标宽度（可选，保持比例）
    :param height: 目标高度（可选，保持比例）
    :return: PIL Image对象
    """
    # 解析SVG
    drawing = svg2rlg(svg_path)
    # 设置尺寸（可选）
    if width and height:
        drawing.width = width
        drawing.height = height
    # 渲染为PNG（字节流）
    png_bytes = io.BytesIO()
    renderPM.drawToFile(drawing, png_bytes, fmt="PNG", dpi=96)
    # 转换为PIL Image
    png_bytes.seek(0)
    img = Image.open(png_bytes).convert("RGB")
    return img


def simulate_img_draw(draw, pos, src, border_width, fill="black"):
    """优化：支持SVG和普通位图"""
    x, y, w, h = pos["x"], pos["y"], pos["width"], pos["height"]
    # 背景
    draw.rectangle([x, y, x + w, y + h], fill="white")
    # 边框
    draw_border(draw, pos, border_width, fill)

    # 处理SVG
    if src.lower().endswith(".svg"):
        try:
            # 转换SVG为PIL Image并缩放至目标尺寸
            svg_img = svg_to_pil_image(src, width=w * 0.8, height=h * 0.8)
            # 计算居中位置
            img_x = x + (w - svg_img.width) / 2
            img_y = y + (h - svg_img.height) / 2
            # 粘贴到画布
            draw.im.paste(svg_img, (int(img_x), int(img_y)))
            return
        except Exception as e:
            print(f"SVG渲染失败：{e}，使用默认占位符")

    # 普通图片/SVG失败时的占位符（原逻辑）
    img_center_x = x + (w - w * 0.8) / 2
    img_center_y = y + (h - h * 0.8) / 2
    draw.rectangle(
        [img_center_x, img_center_y, img_center_x + w * 0.8, img_center_y + h * 0.8],
        fill="black",
    )
    # 绘制路径文字
    try:
        font = ImageFont.truetype(FONT_PATH, 10)
    except:
        font = ImageFont.load_default()
    draw.text((x + 2, y + 2), f"IMG: {os.path.basename(src)}", fill="black", font=font)


def runtime_display():
    """运行时绘制复杂布局"""
    # 1. 读取参数
    if not os.path.exists(RUNTIME_PARAMS_PATH):
        print(f"错误：参数文件 {RUNTIME_PARAMS_PATH} 不存在，请先运行 parse_html.py")
        return

    with open(RUNTIME_PARAMS_PATH, "r", encoding="utf-8") as f:
        params = json.load(f)
    root_w, root_h = params["root_size"]["width"], params["root_size"]["height"]

    # 2. 创建画布
    img = Image.new("RGB", (root_w, root_h), "white")
    draw = ImageDraw.Draw(img)

    # 3. 绘制静态元素
    for elem in params["static_elements"]:
        pos = elem["pos"]
        if elem["type"] == "empty_box":
            # 空白容器+边框
            draw.rectangle(
                [pos["x"], pos["y"], pos["x"] + pos["width"], pos["y"] + pos["height"]],
                fill="white",
            )
            draw_border(draw, pos, elem["style"]["border_width"])

        elif elem["type"] == "text":
            # 静态文字
            draw_text_with_wrap(
                draw,
                elem["content"],
                pos={"x": pos["x"], "y": pos["y"]},
                font_size=int(elem["style"]["font_size"]),
                line_height=int(elem["style"]["line_height"]),
                max_width=pos["width"],
            )

        elif elem["type"] == "img":
            # 静态图片
            simulate_img_draw(
                draw, pos, elem["src"], border_width=elem["style"]["border_width"]
            )

    # 4. 绘制动态元素
    for elem in params["dynamic_elements"]:
        pos = elem["pos"]
        # 处理display控制
        if elem["type"] == "img":
            display_key = (
                elem["display_placeholders"][0]
                if elem["display_placeholders"]
                else None
            )
            display = DYNAMIC_PARAMS.get(
                display_key, elem.get("default_display", "block")
            )
            if display == "none":
                continue

        if elem["type"] == "text":
            # 动态文字
            text = DYNAMIC_PARAMS.get(elem["placeholders"][0], elem["default_content"])
            draw_text_with_wrap(
                draw,
                text,
                pos={"x": pos["x"], "y": pos["y"]},
                font_size=int(elem["style"]["font_size"]),
                line_height=int(elem["style"]["line_height"]),
                max_width=pos["width"],
            )

        elif elem["type"] == "img":
            # 动态图片
            src_key = elem["src_placeholders"][0] if elem["src_placeholders"] else None
            src = DYNAMIC_PARAMS.get(src_key, elem["default_src"])
            simulate_img_draw(
                draw, pos, src, border_width=elem["style"]["border_width"]
            )

    # 5. 保存+显示
    img.save("embedded_display_complex.png")
    img.show()
    print("绘制完成！已保存为 embedded_display_complex.png，同时弹出预览窗口")


if __name__ == "__main__":
    runtime_display()
