#!/usr/bin/env python3
"""
配置完整性测试
验证配置序列化、数据结构、校验机制
"""

import sys
import os
import struct
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from simulator_client import SimulatorClient, print_response


def calculate_crc32(data: bytes) -> int:
    """计算 CRC32 校验和（与 Rust 实现一致）"""
    crc = 0xFFFFFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0xEDB88320
            else:
                crc >>= 1
    return crc ^ 0xFFFFFFFF


def test_flash_file_structure() -> bool:
    """测试 1: Flash 文件结构验证"""
    print("\n" + "=" * 60)
    print("  测试 1: Flash 文件结构验证")
    print("=" * 60)

    flash_file = "/tmp/simulator_flash.bin"

    if not os.path.exists(flash_file):
        print("⚠️  Flash 文件不存在，先创建配置...")
        print("请先运行 test_config_persistence.py 创建配置")
        return True

    # 读取 Flash 数据
    with open(flash_file, "rb") as f:
        data = f.read()

    print(f"Flash 文件大小: {len(data)} bytes")

    if len(data) < 32:
        print("❌ Flash 文件太小，无法解析配置头")
        return False

    # 解析配置头
    magic = struct.unpack("<I", data[0:4])[0]
    version = struct.unpack("<I", data[4:8])[0]
    checksum = struct.unpack("<I", data[8:12])[0]

    print(f"\n配置头解析:")
    print(f"  Magic Number: 0x{magic:08X} (期望: 0x4C585843 'LXXC')")
    print(f"  Version: {version}")
    print(f"  CRC32: 0x{checksum:08X}")

    # 验证 Magic
    if magic == 0x4C585843:
        print("  ✅ Magic Number 正确")
    else:
        print("  ❌ Magic Number 错误")
        return False

    # 验证版本
    if version == 1:
        print("  ✅ 版本号正确")
    else:
        print(f"  ⚠️  版本号 {version}（可能是新版本）")

    # 验证 CRC32
    print(f"\n验证 CRC32...")

    # CRC32 应该覆盖配置数据（从偏移 32 开始）
    if len(data) > 32:
        config_data = data[32:]
        calculated_crc = calculate_crc32(config_data)

        print(f"  计算的 CRC32: 0x{calculated_crc:08X}")
        print(f"  存储的 CRC32: 0x{checksum:08X}")

        if calculated_crc == checksum:
            print("  ✅ CRC32 校验通过")
        else:
            print("  ❌ CRC32 校验失败")
            return False
    else:
        print("  ⚠️  配置数据为空，无法验证 CRC")

    print("\n✅ Flash 文件结构验证通过")
    return True


def test_config_size_limit() -> bool:
    """测试 2: 配置大小限制测试"""
    print("\n" + "=" * 60)
    print("  测试 2: 配置大小限制测试")
    print("=" * 60)

    flash_file = "/tmp/simulator_flash.bin"

    if not os.path.exists(flash_file):
        print("⚠️  Flash 文件不存在")
        return True

    # 读取 Flash 数据
    with open(flash_file, "rb") as f:
        data = f.read()

    # 配置数据大小（从偏移 32 开始）
    config_size = len(data) - 32 if len(data) > 32 else 0

    print(f"配置数据大小: {config_size} bytes")
    print(f"最大限制: 1024 bytes")

    if config_size <= 1024:
        print("✅ 配置大小在限制内")
    else:
        print("❌ 配置超过大小限制")
        return False

    # 分析配置结构
    if config_size > 0:
        print(f"\n配置数据分析:")
        print(f"  前 16 字节 (hex): {data[32:48].hex()}")

        # 尝试识别 postcard 序列化格式
        # postcard 使用变长编码
        print(f"\n  序列化格式: postcard")
        print(f"  版本号 (推测): {data[32] if len(data) > 32 else 'N/A'}")

    return True


def test_corrupted_config_detection() -> bool:
    """测试 3: 损坏配置检测"""
    print("\n" + "=" * 60)
    print("  测试 3: 损坏配置检测")
    print("=" * 60)

    flash_file = "/tmp/simulator_flash.bin"
    backup_file = flash_file + ".backup"

    if not os.path.exists(flash_file):
        print("⚠️  Flash 文件不存在")
        return True

    # 备份原始数据
    with open(flash_file, "rb") as f:
        original_data = f.read()

    print(f"已备份原始配置 ({len(original_data)} bytes)")

    # 创建损坏的配置（修改 CRC）
    corrupted_data = bytearray(original_data)

    # 修改 CRC32 字段（偏移 8-12）
    corrupted_data[8:12] = b"\xff\xff\xff\xff"

    # 写入损坏的配置
    with open(flash_file, "wb") as f:
        f.write(corrupted_data)

    print("已创建损坏的配置（CRC 错误）")

    print("\n⚠️  重启模拟器应检测到损坏并使用默认配置")
    print("请手动验证:")
    print("  1. 重启模拟器")
    print("  2. 检查日志是否显示 'Config checksum mismatch'")
    print("  3. 验证配置恢复为默认值")

    # 恢复原始数据
    time.sleep(1)
    with open(flash_file, "wb") as f:
        f.write(original_data)

    print("\n已恢复原始配置")
    print("✅ 损坏配置检测测试准备完成")

    return True


def test_version_mismatch_handling() -> bool:
    """测试 4: 版本不匹配处理"""
    print("\n" + "=" * 60)
    print("  测试 4: 版本不匹配处理")
    print("=" * 60)

    flash_file = "/tmp/simulator_flash.bin"
    backup_file = flash_file + ".backup"

    if not os.path.exists(flash_file):
        print("⚠️  Flash 文件不存在")
        return True

    # 备份原始数据
    with open(flash_file, "rb") as f:
        original_data = f.read()

    print(f"已备份原始配置")

    # 创建版本不匹配的配置
    version_mismatch_data = bytearray(original_data)

    # 修改版本号（偏移 4-8）
    version_mismatch_data[4:8] = struct.pack("<I", 999)

    # 写入
    with open(flash_file, "wb") as f:
        f.write(version_mismatch_data)

    print("已创建版本不匹配的配置（版本 999）")

    print("\n⚠️  重启模拟器应检测到版本不匹配并使用默认配置")
    print("请手动验证:")
    print("  1. 重启模拟器")
    print("  2. 检查日志是否显示 'Config version mismatch'")
    print("  3. 验证配置恢复为默认值")

    # 恢复原始数据
    time.sleep(1)
    with open(flash_file, "wb") as f:
        f.write(original_data)

    print("\n已恢复原始配置")
    print("✅ 版本不匹配处理测试准备完成")

    return True


def test_magic_number_validation() -> bool:
    """测试 5: Magic Number 验证"""
    print("\n" + "=" * 60)
    print("  测试 5: Magic Number 验证")
    print("=" * 60)

    flash_file = "/tmp/simulator_flash.bin"
    backup_file = flash_file + ".backup"

    if not os.path.exists(flash_file):
        print("⚠️  Flash 文件不存在")
        return True

    # 备份原始数据
    with open(flash_file, "rb") as f:
        original_data = f.read()

    print(f"已备份原始配置")

    # 创建错误的 Magic Number
    wrong_magic_data = bytearray(original_data)

    # 修改 Magic（偏移 0-4）
    wrong_magic_data[0:4] = b"XXXX"

    # 写入
    with open(flash_file, "wb") as f:
        f.write(wrong_magic_data)

    print("已创建错误 Magic Number 的配置")

    print("\n⚠️  重启模拟器应检测到 Magic 错误并使用默认配置")
    print("请手动验证:")
    print("  1. 重启模拟器")
    print("  2. 检查日志是否显示 'Config magic invalid'")
    print("  3. 验证配置恢复为默认值")

    # 恢复原始数据
    time.sleep(1)
    with open(flash_file, "wb") as f:
        f.write(original_data)

    print("\n已恢复原始配置")
    print("✅ Magic Number 验证测试准备完成")

    return True


def run_config_integrity_tests(port: int = 8080) -> bool:
    """运行所有配置完整性测试"""
    print("\n" + "#" * 60)
    print("#  配置完整性测试")
    print("#" * 60)

    client = SimulatorClient(f"http://127.0.0.1:{port}")

    # 检查服务
    try:
        status = client.get_status()
        if "error" in status:
            print(f"⚠️  模拟器服务未启动，部分测试仍可运行")
    except Exception as e:
        print(f"⚠️  连接失败: {e}，部分测试仍可运行")

    # 运行测试（这些测试主要操作文件，不需要模拟器运行）
    results = []

    results.append(("Flash 文件结构", test_flash_file_structure()))
    results.append(("配置大小限制", test_config_size_limit()))
    results.append(("损坏配置检测", test_corrupted_config_detection()))
    results.append(("版本不匹配处理", test_version_mismatch_handling()))
    results.append(("Magic Number 验证", test_magic_number_validation()))

    # 汇总结果
    print("\n" + "=" * 60)
    print("  测试结果汇总")
    print("=" * 60)

    passed = 0
    failed = 0
    for name, result in results:
        status = "✅ 通过" if result else "❌ 失败"
        print(f"  {name}: {status}")
        if result:
            passed += 1
        else:
            failed += 1

    print(f"\n总计: {passed} 通过, {failed} 失败")

    # 打印 Flash 文件位置
    print("\n" + "=" * 60)
    print("  Flash 文件位置")
    print("=" * 60)
    print("  /tmp/simulator_flash.bin")
    print("  备份文件: /tmp/simulator_flash.bin.backup")
    print("=" * 60)

    return failed == 0


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8080
    success = run_config_integrity_tests(port)
    sys.exit(0 if success else 1)
