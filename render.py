#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
渲染800x480嵌入式面板为PNG图片
参数：--input output.py --output output.png
"""
import argparse
import importlib.util
from PIL import Image, ImageDraw, ImageFont


# ===================== 渲染函数 =====================
def render_panel(config_path, output_path):
    """渲染面板为PNG图片"""
    # 1. 导入配置文件
    spec = importlib.util.spec_from_file_location("config", config_path)
    config = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(config)

    # 2. 创建画布
    img = Image.new("RGB", (config.CANVAS_WIDTH, config.CANVAS_HEIGHT), "white")
    draw = ImageDraw.Draw(img)

    # 3. 加载字体（处理字体路径）
    try:
        font_base = ImageFont.truetype(config.FONT_PATH, 16)  # 基础字体
    except:
        # 回退到默认字体
        font_base = ImageFont.load_default()
        print("⚠️  无法加载自定义字体，使用默认字体")

    # 4. 绘制元素
    ## 4.1 绘制分割线
    # 横向分割线1
    div1 = config.ELEMENT_POSITIONS["DIVIDER_1"]
    draw.rectangle(
        [div1["x"], div1["y"], div1["x"] + div1["width"], div1["y"] + div1["height"]],
        fill="black",
    )
    # 纵向分割线
    v_div = config.ELEMENT_POSITIONS["VERTICAL_DIVIDER"]
    draw.rectangle(
        [
            v_div["x"],
            v_div["y"],
            v_div["x"] + v_div["width"],
            v_div["y"] + v_div["height"],
        ],
        fill="black",
    )

    ## 4.2 绘制时间数字（图片占位框）
    time_digits = [
        "TIME_DIGIT_1",
        "TIME_DIGIT_2",
        "TIME_DIGIT_COLON",
        "TIME_DIGIT_3",
        "TIME_DIGIT_4",
    ]
    for digit_name in time_digits:
        pos = config.ELEMENT_POSITIONS[digit_name]
        # 绘制图片占位框（矩形）
        draw.rectangle(
            [pos["x"], pos["y"], pos["x"] + pos["width"], pos["y"] + pos["height"]],
            outline="black",
            width=1,
        )
        # 标注占位符（示例值）
        if digit_name == "TIME_DIGIT_1":
            text = config.MOCK_DATA["time_digit_hour_tens"]
        elif digit_name == "TIME_DIGIT_2":
            text = config.MOCK_DATA["time_digit_hour_ones"]
        elif digit_name == "TIME_DIGIT_COLON":
            text = ":"
        elif digit_name == "TIME_DIGIT_3":
            text = config.MOCK_DATA["time_digit_minute_tens"]
        elif digit_name == "TIME_DIGIT_4":
            text = config.MOCK_DATA["time_digit_minute_ones"]

        # 居中绘制文字
        font = (
            ImageFont.truetype(config.FONT_PATH, 60) if config.FONT_PATH else font_base
        )
        text_bbox = draw.textbbox((0, 0), text, font=font)
        text_width = text_bbox[2] - text_bbox[0]
        text_height = text_bbox[3] - text_bbox[1]
        text_x = pos["x"] + (pos["width"] - text_width) / 2
        text_y = pos["y"] + (pos["height"] - text_height) / 2
        draw.text((text_x, text_y), text, fill="black", font=font)

    ## 4.3 绘制日期文本
    date_pos = config.ELEMENT_POSITIONS["DATE_WRAP"]
    date_font = (
        ImageFont.truetype(config.FONT_PATH, date_pos["font_size"])
        if config.FONT_PATH
        else font_base
    )
    draw.text(
        (date_pos["x"], date_pos["y"]),
        config.MOCK_DATA["date"],
        fill="black",
        font=date_font,
    )

    ## 4.4 绘制农历元素
    # 农历年
    lunar_year_pos = config.ELEMENT_POSITIONS["LUNAR_YEAR"]
    lunar_year_font = ImageFont.truetype(config.FONT_PATH, lunar_year_pos["font_size"])
    draw.text(
        (lunar_year_pos["x"], lunar_year_pos["y"]),
        config.MOCK_DATA["lunar_year"],
        fill="black",
        font=lunar_year_font,
    )
    # 农历日
    lunar_day_pos = config.ELEMENT_POSITIONS["LUNAR_DAY"]
    lunar_day_font = ImageFont.truetype(config.FONT_PATH, lunar_day_pos["font_size"])
    draw.text(
        (lunar_day_pos["x"], lunar_day_pos["y"]),
        config.MOCK_DATA["lunar_day"],
        fill="black",
        font=lunar_day_font,
    )
    # 农历宜忌
    lunar_yi_ji_pos = config.ELEMENT_POSITIONS["LUNAR_YI_JI"]
    lunar_yi_ji_font = ImageFont.truetype(
        config.FONT_PATH, lunar_yi_ji_pos["font_size"]
    )
    draw.text(
        (lunar_yi_ji_pos["x"], lunar_yi_ji_pos["y"]),
        f"宜：{config.MOCK_DATA['lunar_suitable']}\n忌：{config.MOCK_DATA['lunar_avoid']}",
        fill="black",
        font=lunar_yi_ji_font,
    )

    ## 4.5 绘制天气元素
    # 天气位置
    weather_loc_pos = config.ELEMENT_POSITIONS["WEATHER_LOCATION"]
    weather_loc_font = ImageFont.truetype(
        config.FONT_PATH, weather_loc_pos["font_size"]
    )
    draw.text(
        (weather_loc_pos["x"], weather_loc_pos["y"]),
        config.MOCK_DATA["weather_location"],
        fill="black",
        font=weather_loc_font,
    )
    # 温湿度
    weather_temp_pos = config.ELEMENT_POSITIONS["WEATHER_TEMP_HUM"]
    weather_temp_font = ImageFont.truetype(
        config.FONT_PATH, weather_temp_pos["font_size"]
    )
    draw.text(
        (weather_temp_pos["x"], weather_temp_pos["y"]),
        config.MOCK_DATA["weather_temp_hum"],
        fill="black",
        font=weather_temp_font,
    )
    # 3天天气
    for i in range(1, 4):
        # 日期文本
        day_pos = config.ELEMENT_POSITIONS[f"WEATHER_DAY_{i}"]
        day_font = ImageFont.truetype(config.FONT_PATH, day_pos["font_size"])
        draw.text(
            (day_pos["x"], day_pos["y"]),
            config.MOCK_DATA[f"day{i}"],
            fill="black",
            font=day_font,
        )
        # 天气描述
        draw.text(
            (day_pos["x"], day_pos["y"] + 50),
            config.MOCK_DATA[f"desc{i}"],
            fill="black",
            font=day_font,
        )
        # 天气图标占位框
        icon_pos = config.ELEMENT_POSITIONS[f"WEATHER_ICON_{i}"]
        draw.rectangle(
            [
                icon_pos["x"],
                icon_pos["y"],
                icon_pos["x"] + icon_pos["width"],
                icon_pos["y"] + icon_pos["height"],
            ],
            outline="black",
            width=1,
        )

    ## 4.6 绘制格言
    # 格言内容
    motto_content_pos = config.ELEMENT_POSITIONS["MOTTO_CONTENT"]
    motto_content_font = ImageFont.truetype(
        config.FONT_PATH, motto_content_pos["font_size"]
    )
    draw.text(
        (motto_content_pos["x"], motto_content_pos["y"]),
        config.MOCK_DATA["motto_content"],
        fill="black",
        font=motto_content_font,
    )
    # 格言来源
    motto_source_pos = config.ELEMENT_POSITIONS["MOTTO_SOURCE"]
    motto_source_font = ImageFont.truetype(
        config.FONT_PATH, motto_source_pos["font_size"]
    )
    draw.text(
        (motto_source_pos["x"], motto_source_pos["y"]),
        config.MOCK_DATA["motto_source"],
        fill="black",
        font=motto_source_font,
    )

    ## 4.7 绘制状态图标（占位框）
    status_icons = ["NETWORK_ICON", "BATTERY_ICON", "CHARGING_ICON"]
    for icon_name in status_icons:
        pos = config.ELEMENT_POSITIONS[icon_name]
        draw.rectangle(
            [pos["x"], pos["y"], pos["x"] + pos["width"], pos["y"] + pos["height"]],
            outline="black",
            width=1,
        )
        # 标注图标名称
        draw.text(
            (pos["x"], pos["y"] + pos["height"] + 2),
            icon_name.replace("_", " "),
            fill="gray",
            font=font_base,
        )

    # 5. 保存图片
    img.save(output_path)
    print(f"✅ 渲染完成！已保存为 {output_path}")


# ===================== 主函数 =====================
def main():
    # 解析命令行参数
    parser = argparse.ArgumentParser(description="渲染800x480嵌入式面板为PNG")
    parser.add_argument("--input", default="output.py", help="输入配置文件路径")
    parser.add_argument("--output", default="output.png", help="输出PNG图片路径")
    args = parser.parse_args()

    # 执行渲染
    render_panel(args.input, args.output)


if __name__ == "__main__":
    main()
