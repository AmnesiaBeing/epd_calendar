#!/usr/bin/env python3
"""
墨水屏字体显示诊断工具
"""

import sys
import argparse


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


def diagnose_font_issue(bitmap_data, char_width, char_height):
    """诊断可能的显示问题"""
    bytes_per_row = (char_width + 7) // 8

    print("=== 诊断信息 ===")

    # 1. 检查字节序问题
    print("1. 字节序测试:")
    print("   MSB优先 (当前):")
    print_bitmap_msb(bitmap_data, char_width, char_height)
    print("   LSB优先 (备选):")
    print_bitmap_lsb(bitmap_data, char_width, char_height)

    # 2. 检查颜色反转
    print("2. 颜色反转测试:")
    print("   正常:")
    print_bitmap_msb(bitmap_data, char_width, char_height)
    print("   反转:")
    inverted_data = [~b & 0xFF for b in bitmap_data]
    print_bitmap_msb(inverted_data, char_width, char_height)

    # 3. 检查数据对齐
    print("3. 数据对齐检查:")
    check_data_alignment(bitmap_data, char_width, char_height)


def print_bitmap_msb(char_data, char_width, char_height):
    """MSB优先显示"""
    bytes_per_row = (char_width + 7) // 8
    print("+{}+".format("-" * char_width))
    for y in range(char_height):
        line = "|"
        for x in range(char_width):
            byte_index = y * bytes_per_row + x // 8
            bit_offset = 7 - (x % 8)  # MSB优先
            pixel = (
                (char_data[byte_index] >> bit_offset) & 1
                if byte_index < len(char_data)
                else 0
            )
            line += "█" if pixel else " "
        line += "|"
        print(line)
    print("+{}+".format("-" * char_width))


def print_bitmap_lsb(char_data, char_width, char_height):
    """LSB优先显示"""
    bytes_per_row = (char_width + 7) // 8
    print("+{}+".format("-" * char_width))
    for y in range(char_height):
        line = "|"
        for x in range(char_width):
            byte_index = y * bytes_per_row + x // 8
            bit_offset = x % 8  # LSB优先
            pixel = (
                (char_data[byte_index] >> bit_offset) & 1
                if byte_index < len(char_data)
                else 0
            )
            line += "█" if pixel else " "
        line += "|"
        print(line)
    print("+{}+".format("-" * char_width))


def check_data_alignment(char_data, char_width, char_height):
    """检查数据对齐"""
    bytes_per_row = (char_width + 7) // 8
    expected_size = bytes_per_row * char_height

    print(f"   字符尺寸: {char_width}x{char_height}")
    print(f"   每行字节: {bytes_per_row}")
    print(f"   预期大小: {expected_size} 字节")
    print(f"   实际大小: {len(char_data)} 字节")

    if len(char_data) != expected_size:
        print("   ⚠️ 数据大小不匹配!")
    else:
        print("   ✓ 数据大小正确")


def main():
    parser = argparse.ArgumentParser(description="墨水屏字体诊断工具")
    parser.add_argument("file", help="bin文件路径")
    parser.add_argument("--width", type=int, required=True, help="字符宽度")
    parser.add_argument("--height", type=int, required=True, help="字符高度")
    parser.add_argument("--index", type=int, default=0, help="字符索引")

    args = parser.parse_args()

    # 读取字符数据（使用之前提供的 read_font_bin 函数）
    char_data = read_font_bin(args.file, args.width, args.height, args.index)

    print(f"诊断字符 #{args.index}:")
    diagnose_font_issue(char_data, args.width, args.height)


if __name__ == "__main__":
    main()
