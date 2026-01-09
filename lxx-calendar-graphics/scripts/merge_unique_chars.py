#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
读取命令行指定的多个文本文件，合并内容后按字符级别去重、排序，输出唯一字符集
编码格式：UTF-8
使用方式：python script_name.py 文件1.txt 文件2.txt ...
"""

import sys


def read_files_and_collect_chars(file_paths):
    """
    读取多个文件，收集所有字符（去重）
    :param file_paths: 文本文件路径列表
    :return: 去重后的字符集合
    """
    char_set = set()  # 集合自动去重，存储所有唯一字符

    for file_path in file_paths:
        try:
            # 以UTF-8编码打开文件，读取全部内容
            with open(file_path, "r", encoding="utf-8") as f:
                content = f.read()
                # 将每个字符添加到集合（自动去重）
                for char in content:
                    char_set.add(char)
            print(f"✅ 成功处理文件：{file_path}")

        except FileNotFoundError:
            print(f"❌ 错误：文件 {file_path} 不存在")
        except PermissionError:
            print(f"❌ 错误：没有权限读取文件 {file_path}")
        except UnicodeDecodeError:
            print(f"❌ 错误：文件 {file_path} 不是UTF-8编码，请检查文件编码")
        except Exception as e:
            print(f"❌ 处理文件 {file_path} 时发生未知错误：{str(e)}")

    return char_set


def main():
    # 检查命令行参数：至少需要指定一个文件
    if len(sys.argv) < 2:
        print("📚 使用方法：")
        print(f"   python {sys.argv[0]} 文件1.txt [文件2.txt] [文件3.txt] ...")
        sys.exit(1)

    # 获取命令行输入的文件路径列表（排除脚本名本身）
    file_paths = sys.argv[1:]

    # 读取文件并收集唯一字符
    unique_chars = read_files_and_collect_chars(file_paths)

    if not unique_chars:
        print("⚠️  未收集到任何有效字符")
        sys.exit(0)

    # 对唯一字符进行排序（按Unicode码点排序，保证顺序稳定）
    sorted_chars = sorted(unique_chars)

    # 输出结果
    print("\n========================================")
    print(f"📊 统计结果：共收集到 {len(sorted_chars)} 个唯一字符")
    print("🔤 排序后的唯一字符列表：")
    print("----------------------------------------")
    # 方式1：逐个打印（带索引，便于查看）
    # for idx, char in enumerate(sorted_chars, 1):
    #     # 对不可见字符（如换行、制表符）进行转义显示
    #     repr_char = repr(char) if char in ["\n", "\r", "\t", "\b", "\f"] else char
    #     print(f"{idx:3d}: {repr_char} (Unicode: U+{ord(char):04X})")

    # 方式2：拼接成字符串输出（可选，取消注释即可）
    # sorted_str = ''.join(sorted_chars)
    # print("\n拼接后的唯一字符字符串：")
    # print(sorted_str)

    # 可选：将结果保存到文件（取消注释即可）
    output_file = "unique_chars_result.txt"
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write(''.join(sorted_chars))
    print(f"\n💾 结果已保存到文件：{output_file}")


if __name__ == "__main__":
    main()
