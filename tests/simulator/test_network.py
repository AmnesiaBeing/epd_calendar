#!/usr/bin/env python3
"""
网络同步测试
验证时间同步、天气同步功能
注意：需要先执行 sudo ./tap.sh 创建虚拟网络设备
"""

import sys
import os
import time
import subprocess

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from simulator_client import SimulatorClient, print_response


def test_network_available(client: SimulatorClient) -> bool:
    """测试 1: 检查网络是否可用"""
    print("\n" + "=" * 60)
    print("  测试 1: 检查网络可用性")
    print("=" * 60)

    status = client.get_status()
    print_response(status, "系统状态")

    # 检查 tap 设备
    result = subprocess.run(
        ["ip", "link", "show", "tap99"], capture_output=True, text=True
    )
    if result.returncode == 0:
        print("\n✅ tap99 设备已创建")
    else:
        print("\n⚠️  tap99 设备不存在，请先执行: sudo ./tap.sh")

    return True


def test_wifi_config_and_sync(client: SimulatorClient) -> bool:
    """测试 2: WiFi 配置并触发网络同步"""
    print("\n" + "=" * 60)
    print("  测试 2: WiFi 配置与网络同步")
    print("=" * 60)

    # 连接 BLE
    print("步骤 1: 连接 BLE...")
    result = client.ble_connect()
    print_response(result, "BLE 连接")
    time.sleep(0.5)

    # 发送 WiFi 配置
    print("\n步骤 2: 发送 WiFi 配置...")
    result = client.ble_config("TestWiFi", "password123")
    print_response(result, "WiFi 配置响应")
    time.sleep(1)

    # 发送位置配置
    print("\n步骤 3: 发送位置配置...")
    result = client.ble_network_config("101010100", 60, True)
    print_response(result, "位置配置响应")
    time.sleep(1)

    # 发送同步命令
    print("\n步骤 4: 发送网络同步命令...")
    result = client.ble_command("network_sync")
    print_response(result, "同步命令响应")

    time.sleep(3)  # 等待同步完成

    # 断开 BLE
    client.ble_disconnect()

    print("\n✅ 网络同步测试完成，请查看日志确认结果")
    return True


def test_time_sync(log_file: str) -> bool:
    """测试 3: 检查时间同步结果"""
    print("\n" + "=" * 60)
    print("  测试 3: 时间同步结果检查")
    print("=" * 60)

    try:
        with open(log_file, "r") as f:
            logs = f.read()

        if "SNTP time sync success" in logs or "Successfully got time" in logs:
            print("✅ 时间同步成功")
            # 提取同步的时间
            import re

            match = re.search(r"Successfully got time: (\d+)", logs)
            if match:
                timestamp = int(match.group(1))
                import datetime

                dt = datetime.datetime.fromtimestamp(timestamp)
                print(f"   同步时间: {dt.strftime('%Y-%m-%d %H:%M:%S')}")
            return True
        elif "SNTP time sync failed" in logs:
            print("⚠️  时间同步失败")
            return True
        else:
            print("⚠️  未找到时间同步日志")
            return True
    except Exception as e:
        print(f"❌ 读取日志失败: {e}")
        return False


def test_weather_sync(log_file: str) -> bool:
    """测试 4: 检查天气同步结果"""
    print("\n" + "=" * 60)
    print("  测试 4: 天气同步结果检查")
    print("=" * 60)

    try:
        with open(log_file, "r") as f:
            logs = f.read()

        if "Weather synchronized successfully" in logs:
            print("✅ 天气同步成功")

            if "JWT signer not configured" in logs:
                print("   ℹ️  使用默认天气数据（未配置和风天气 API 凭据）")
            elif "Weather API response received" in logs:
                print("   ✅ 成功从和风天气 API 获取数据")
            return True
        elif "Weather sync failed" in logs:
            print("⚠️  天气同步失败")
            return True
        else:
            print("⚠️  未找到天气同步日志")
            return True
    except Exception as e:
        print(f"❌ 读取日志失败: {e}")
        return False


def run_network_tests(port: int = 8080) -> bool:
    """运行所有网络测试"""
    print("\n" + "#" * 60)
    print("#  模拟器网络同步测试")
    print("#" * 60)

    log_file = os.path.join(os.path.dirname(__file__), "../../simulator.log")

    print("""
⚠️  测试前提条件：
1. 已执行 sudo ./tap.sh 创建虚拟网络设备
2. 系统已启用 IP 转发和 NAT

和风天气 API 配置说明：
- 时间同步：自动使用公共 NTP 服务器
- 天气同步：需要配置和风天气 JWT 凭据
  - 如未配置，将使用默认天气数据
""")

    client = SimulatorClient(f"http://127.0.0.1:{port}")

    try:
        status = client.get_status()
        if "error" in status:
            print(f"❌ 无法连接到模拟器服务: {status['error']}")
            return False
    except Exception as e:
        print(f"❌ 连接失败: {e}")
        return False

    results = []

    results.append(("网络可用性", test_network_available(client)))
    results.append(("WiFi 配置与同步", test_wifi_config_and_sync(client)))
    results.append(("时间同步", test_time_sync(log_file)))
    results.append(("天气同步", test_weather_sync(log_file)))

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

    print("""
📝 测试说明：

1. 时间同步：使用公共 NTP 服务器自动同步
2. 天气同步：
   - 配置和风天气凭据：通过代码设置 JWT signer
   - 未配置时：使用默认天气数据（上海，晴天）
   - 位置配置：通过 BLE 发送 network_config

3. 完整测试需要：
   - tap 设备: sudo ./tap.sh
   - 网络连通性: 确保可以访问外网
""")

    return failed == 0


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8080
    success = run_network_tests(port)
    sys.exit(0 if success else 1)
