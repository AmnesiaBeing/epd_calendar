#!/usr/bin/env python3
"""
字体bin文件可视化工具
用于调试嵌入式墨水屏系统的字体显示问题
"""

import sys
import os
import argparse
from pathlib import Path


def read_font_bin(file_path, char_width, char_height, char_index=0):
    """
    从bin文件中读取指定字符的点阵数据

    Args:
        file_path: bin文件路径
        char_width: 字符宽度（像素）
        char_height: 字符高度（像素）
        char_index: 字符索引（从0开始）

    Returns:
        bytes: 字符的点阵数据
    """
    # 计算每行需要的字节数（向上取整到8的倍数）
    bytes_per_row = (char_width + 7) // 8
    # 每个字符的总字节数
    char_size = bytes_per_row * char_height

    with open(file_path, "rb") as f:
        data = f.read()

    # 计算字符在文件中的偏移量
    offset = char_index * char_size

    if offset + char_size > len(data):
        raise ValueError(f"字符索引 {char_index} 超出文件范围")

    return data[offset : offset + char_size]


def bytes_to_bitmap(char_data, char_width, char_height):
    """
    将字节数据转换为二维位图

    Args:
        char_data: 字符的字节数据
        char_width: 字符宽度
        char_height: 字符高度
        char_index: 字符索引

    Returns:
        list: 二维列表表示的位图，True表示黑色像素
    """
    bytes_per_row = (char_width + 7) // 8
    bitmap = [[False] * char_width for _ in range(char_height)]

    for y in range(char_height):
        row_start = y * bytes_per_row
        for x in range(char_width):
            byte_index = row_start + x // 8
            bit_offset = 7 - (x % 8)  # MSB优先

            if byte_index < len(char_data):
                if char_data[byte_index] & (1 << bit_offset):
                    bitmap[y][x] = True

    return bitmap


def print_bitmap(bitmap, char_width, char_height):
    """
    打印位图到控制台

    Args:
        bitmap: 二维位图数据
        char_width: 字符宽度
        char_height: 字符高度
    """
    print(f"字符点阵 ({char_width}×{char_height}):")
    print("+" + "-" * char_width + "+")
    for y in range(char_height):
        print("|", end="")
        for x in range(char_width):
            print("█" if bitmap[y][x] else " ", end="")
        print("|")
    print("+" + "-" * char_width + "+")


def save_bitmap_as_ascii(bitmap, char_width, char_height, output_file):
    """
    将位图保存为ASCII艺术文件

    Args:
        bitmap: 二维位图数据
        char_width: 字符宽度
        char_height: 字符高度
        output_file: 输出文件路径
    """
    with open(output_file, "w", encoding="utf-8") as f:
        f.write(f"字符点阵 ({char_width}×{char_height}):\n")
        f.write("+" + "-" * char_width + "+\n")
        for y in range(char_height):
            f.write("|")
            for x in range(char_width):
                f.write("█" if bitmap[y][x] else " ")
            f.write("|\n")
        f.write("+" + "-" * char_width + "+\n")


def save_bitmap_as_binary(bitmap, char_width, char_height, output_file):
    """
    将位图保存为二进制格式文件（用于进一步分析）

    Args:
        bitmap: 二维位图数据
        char_width: 字符宽度
        char_height: 字符高度
        output_file: 输出文件路径
    """
    with open(output_file, "w", encoding="utf-8") as f:
        f.write(f"字符点阵二进制数据 ({char_width}×{char_height}):\n")
        for y in range(char_height):
            for x in range(char_width):
                f.write("1" if bitmap[y][x] else "0")
            f.write("\n")


def analyze_font_file(file_path, char_width, char_height):
    """
    分析字体文件的基本信息

    Args:
        file_path: bin文件路径
        char_width: 字符宽度
        char_height: 字符高度
    """
    bytes_per_row = (char_width + 7) // 8
    char_size = bytes_per_row * char_height

    with open(file_path, "rb") as f:
        data = f.read()

    file_size = len(data)
    char_count = file_size // char_size

    print(f"文件: {file_path}")
    print(f"文件大小: {file_size} 字节")
    print(f"字符尺寸: {char_width}×{char_height} 像素")
    print(f"每字符字节数: {char_size} 字节")
    print(f"字符总数: {char_count}")
    print(f"预计字符数: {file_size // char_size}")

    # 检查文件是否完整
    if file_size % char_size != 0:
        print(f"⚠️  警告: 文件大小不是字符大小的整数倍，可能不完整")

    return char_count


def main():
    parser = argparse.ArgumentParser(description="字体bin文件可视化工具")
    parser.add_argument("file", help="bin文件路径")
    parser.add_argument("--width", type=int, required=True, help="字符宽度（像素）")
    parser.add_argument("--height", type=int, required=True, help="字符高度（像素）")
    parser.add_argument("--index", type=int, default=0, help="字符索引（默认: 0）")
    parser.add_argument(
        "--analyze", action="store_true", help="仅分析文件信息，不显示字符"
    )
    parser.add_argument("--output-ascii", help="将字符点阵保存为ASCII文件")
    parser.add_argument("--output-binary", help="将字符点阵保存为二进制文本文件")
    parser.add_argument("--list", type=int, help="显示前N个字符的预览")

    args = parser.parse_args()

    if not os.path.exists(args.file):
        print(f"错误: 文件不存在: {args.file}")
        return 1

    try:
        # 分析文件
        char_count = analyze_font_file(args.file, args.width, args.height)

        if args.analyze:
            return 0

        if args.list:
            # 显示前N个字符的预览
            print(f"\n显示前 {args.list} 个字符的预览:")
            for i in range(min(args.list, char_count)):
                print(f"\n字符 #{i}:")
                char_data = read_font_bin(args.file, args.width, args.height, i)
                bitmap = bytes_to_bitmap(char_data, args.width, args.height)
                print_bitmap(bitmap, args.width, args.height)
        else:
            # 显示单个字符
            print(f"\n显示字符 #{args.index}:")
            char_data = read_font_bin(args.file, args.width, args.height, args.index)

            # 显示原始字节数据（用于调试）
            print("原始字节数据:")
            bytes_per_row = (args.width + 7) // 8
            for y in range(args.height):
                row_start = y * bytes_per_row
                row_end = row_start + bytes_per_row
                row_bytes = char_data[row_start:row_end]
                hex_str = " ".join([f"{b:02x}" for b in row_bytes])
                print(f"行 {y:2d}: {hex_str}")

            bitmap = bytes_to_bitmap(char_data, args.width, args.height)
            print_bitmap(bitmap, args.width, args.height)

            # 保存输出文件
            if args.output_ascii:
                save_bitmap_as_ascii(bitmap, args.width, args.height, args.output_ascii)
                print(f"ASCII点阵已保存到: {args.output_ascii}")

            if args.output_binary:
                save_bitmap_as_binary(
                    bitmap, args.width, args.height, args.output_binary
                )
                print(f"二进制点阵已保存到: {args.output_binary}")

        return 0

    except Exception as e:
        print(f"错误: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
