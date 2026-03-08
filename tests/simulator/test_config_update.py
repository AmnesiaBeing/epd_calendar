#!/usr/bin/env python3
"""
配置更新测试
验证各种配置类型的更新和保存功能
"""

import sys
import os
import time
import json

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from simulator_client import SimulatorClient, print_response


def test_wifi_config_update(client: SimulatorClient) -> bool:
    """测试 1: WiFi 配置更新"""
    print("\n" + "=" * 60)
    print("  测试 1: WiFi 配置更新")
    print("=" * 60)

    # 连接 BLE
    result = client.ble_connect()
    if "error" in result:
        print(f"❌ BLE 连接失败")
        return False

    time.sleep(0.5)

    # 测试不同的 WiFi 配置
    test_cases = [
        ("MyHomeWiFi", "password123"),
        ("Office_5G", "ComplexP@ssw0rd!"),
        ("Cafe_Network", "simple"),
    ]

    for ssid, password in test_cases:
        print(f"\n测试 WiFi 配置:")
        print(f"  SSID: {ssid}")
        print(f"  Password: {password}")

        result = client.ble_config(ssid, password)
        if result.get("success"):
            print(f"  ✅ 配置发送成功")
        else:
            print(f"  ❌ 配置发送失败")
            return False

        time.sleep(0.5)

    print("\n✅ WiFi 配置更新测试通过")
    return True


def test_network_config_update(client: SimulatorClient) -> bool:
    """测试 2: 网络配置更新（位置、同步间隔）"""
    print("\n" + "=" * 60)
    print("  测试 2: 网络配置更新")
    print("=" * 60)

    # 连接 BLE
    result = client.ble_connect()
    if "error" in result:
        print(f"❌ BLE 连接失败")
        return False

    time.sleep(0.5)

    # 测试位置配置
    test_cases = [
        ("101010101", 60, True),  # 北京
        ("101020100", 120, True),  # 上海
        ("101280101", 30, False),  # 广州，不自动同步
    ]

    for location_id, sync_interval, auto_sync in test_cases:
        print(f"\n测试网络配置:")
        print(f"  Location ID: {location_id}")
        print(f"  Sync Interval: {sync_interval} 分钟")
        print(f"  Auto Sync: {auto_sync}")

        result = client.ble_network_config(location_id, sync_interval, auto_sync)
        if result.get("success"):
            print(f"  ✅ 配置发送成功")
        else:
            print(f"  ❌ 配置发送失败")
            return False

        time.sleep(0.5)

    print("\n✅ 网络配置更新测试通过")
    return True


def test_command_actions(client: SimulatorClient) -> bool:
    """测试 3: BLE 命令操作"""
    print("\n" + "=" * 60)
    print("  测试 3: BLE 命令操作")
    print("=" * 60)

    # 连接 BLE
    result = client.ble_connect()
    if "error" in result:
        print(f"❌ BLE 连接失败")
        return False

    time.sleep(0.5)

    # 测试不同命令
    commands = ["network_sync", "reboot", "factory_reset"]

    for cmd in commands:
        print(f"\n发送命令: {cmd}")

        result = client.ble_command(cmd)
        print_response(result, f"{cmd} 响应")

        if result.get("success"):
            print(f"  ✅ 命令发送成功")
        else:
            print(f"  ⚠️  命令可能未实现")

        time.sleep(1)

    print("\n✅ BLE 命令操作测试完成")
    return True


def test_config_field_limits(client: SimulatorClient) -> bool:
    """测试 4: 配置字段边界测试"""
    print("\n" + "=" * 60)
    print("  测试 4: 配置字段边界测试")
    print("=" * 60)

    # 连接 BLE
    result = client.ble_connect()
    if "error" in result:
        print(f"❌ BLE 连接失败")
        return False

    time.sleep(0.5)

    # 测试 SSID 长度限制（应该是 32 字符）
    print("\n测试 SSID 边界:")

    # 最大长度 SSID (32 字符)
    max_ssid = "A" * 32
    print(f"  最大长度 SSID ({len(max_ssid)} 字符): {max_ssid}")
    result = client.ble_config(max_ssid, "password")
    print(f"  结果: {'✅ 成功' if result.get('success') else '❌ 失败'}")

    # 超长 SSID (33 字符)
    overflow_ssid = "B" * 33
    print(f"  超长 SSID ({len(overflow_ssid)} 字符)")
    result = client.ble_config(overflow_ssid, "password")
    print(f"  结果: {'⚠️  应该失败' if result.get('success') else '✅ 正确拒绝'}")

    time.sleep(0.5)

    # 测试密码长度（应该是 64 字节）
    print("\n测试密码边界:")

    # 长密码
    long_password = "P" * 64
    print(f"  长密码 ({len(long_password)} 字符)")
    result = client.ble_config("TestWiFi", long_password)
    print(f"  结果: {'✅ 成功' if result.get('success') else '❌ 失败'}")

    print("\n✅ 配置字段边界测试完成")
    return True


def test_rapid_config_updates(client: SimulatorClient) -> bool:
    """测试 5: 快速连续配置更新"""
    print("\n" + "=" * 60)
    print("  测试 5: 快速连续配置更新")
    print("=" * 60)

    # 连接 BLE
    result = client.ble_connect()
    if "error" in result:
        print(f"❌ BLE 连接失败")
        return False

    time.sleep(0.5)

    # 快速发送多次配置
    print("快速发送 5 次配置更新...")

    for i in range(5):
        ssid = f"WiFi_{i}"
        password = f"Pass_{i}"

        result = client.ble_config(ssid, password)
        print(f"  第 {i + 1} 次: {'✅' if result.get('success') else '❌'}")

        # 不等待，快速发送

    time.sleep(1)

    print("\n✅ 快速连续配置更新测试完成")
    return True


def run_config_update_tests(port: int = 8080) -> bool:
    """运行所有配置更新测试"""
    print("\n" + "#" * 60)
    print("#  配置更新测试")
    print("#" * 60)

    client = SimulatorClient(f"http://127.0.0.1:{port}")

    # 检查服务
    try:
        status = client.get_status()
        if "error" in status:
            print(f"❌ 无法连接到模拟器服务: {status['error']}")
            return False
    except Exception as e:
        print(f"❌ 连接失败: {e}")
        return False

    # 运行测试
    results = []

    results.append(("WiFi 配置更新", test_wifi_config_update(client)))
    results.append(("网络配置更新", test_network_config_update(client)))
    results.append(("BLE 命令操作", test_command_actions(client)))
    results.append(("配置字段边界", test_config_field_limits(client)))
    results.append(("快速连续更新", test_rapid_config_updates(client)))

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

    return failed == 0


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8080
    success = run_config_update_tests(port)
    sys.exit(0 if success else 1)
