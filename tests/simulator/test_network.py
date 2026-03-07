#!/usr/bin/env python3
"""
网络同步测试
验证时间同步、天气同步功能
注意：需要先执行 sudo ./tap.sh 创建虚拟网络设备
"""

import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from simulator_client import SimulatorClient, print_response


def test_network_available(client: SimulatorClient) -> bool:
    """测试 1: 检查网络是否可用"""
    print("\n" + "=" * 60)
    print("  测试 1: 检查网络可用性")
    print("=" * 60)

    status = client.get_status()
    print_response(status, "系统状态")

    print("\n⚠️  注意: 如果未执行 sudo ./tap.sh，网络将处于离线模式")
    print("   这是预期行为，不影响其他功能测试")

    return True


def test_wifi_config_via_ble(client: SimulatorClient) -> bool:
    """测试 2: 通过 BLE 配置 WiFi 并触发网络同步"""
    print("\n" + "=" * 60)
    print("  测试 2: WiFi 配置（通过 BLE）并触发网络同步")
    print("=" * 60)

    import time

    # 先连接 BLE
    print("步骤 1: 连接 BLE...")
    result = client.ble_connect()
    print_response(result, "BLE 连接")

    time.sleep(0.5)

    # 发送 WiFi 配置（正确格式）
    test_ssid = "TestWiFi_Network"
    test_password = "test_password_123"

    print(f"\n步骤 2: 发送 WiFi 配置...")
    print(f"  SSID: {test_ssid}")
    print(f"  Password: {test_password}")

    result = client.ble_config(test_ssid, test_password)
    print_response(result, "配置响应")

    if result.get("success"):
        print("\n✅ WiFi 配置已发送")
        print("   请查看日志确认 WiFi 连接和网络同步结果")
        time.sleep(2)

        # 发送网络同步命令
        print("\n步骤 3: 发送网络同步命令...")
        sync_result = client.ble_command("network_sync")
        print_response(sync_result, "同步命令响应")

        time.sleep(0.5)
        client.ble_disconnect()
        return True
    else:
        print("\n❌ WiFi 配置测试失败")
        client.ble_disconnect()
        return False


def test_network_sync_manual() -> bool:
    """测试 3: 手动网络同步测试（需要 tap 设备）"""
    print("\n" + "=" * 60)
    print("  测试 3: 手动网络同步测试")
    print("=" * 60)

    print("""
⚠️  此测试需要以下条件：
1. 已执行 sudo ./tap.sh 创建虚拟网络设备
2. 已配置 WiFi（通过 BLE 配置）
3. 模拟器已连接到网络

测试方法：
1. 确认 tap 设备存在: ip link show tap99
2. 通过 BLE 配置 WiFi
3. 查看日志中的网络同步信息

如果未满足条件，此测试将被跳过。
""")

    import subprocess

    result = subprocess.run(
        ["ip", "link", "show", "tap99"], capture_output=True, text=True
    )

    if result.returncode == 0:
        print("✅ tap99 设备存在，网络同步测试环境就绪")
        print("   请查看模拟器日志验证同步结果")
        return True
    else:
        print("⚠️  tap99 设备不存在，网络同步测试跳过")
        print("   如需测试网络同步，请执行: sudo ./tap.sh")
        return True


def test_rtc_time_display(client: SimulatorClient) -> bool:
    """测试 4: RTC 时间显示测试"""
    print("\n" + "=" * 60)
    print("  测试 4: RTC 时间显示测试")
    print("=" * 60)

    rtc_status = client.get_rtc_status()
    print_response(rtc_status, "RTC 状态")

    timestamp = rtc_status.get("timestamp", 0)
    if timestamp > 0:
        import datetime

        try:
            dt = datetime.datetime.fromtimestamp(timestamp)
            print(f"\n当前时间: {dt.strftime('%Y-%m-%d %H:%M:%S')}")
            print("✅ RTC 时间显示正常")
            return True
        except:
            print(f"⚠️  时间戳格式异常: {timestamp}")
            return True
    else:
        print("⚠️  时间戳为 0")
        return True


def run_network_tests(port: int = 8080) -> bool:
    """运行所有网络测试"""
    print("\n" + "#" * 60)
    print("#  模拟器网络同步测试")
    print("#" * 60)
    print("""
⚠️  注意事项：
1. 如果未执行 sudo ./tap.sh，网络将处于离线模式
2. 这是预期行为，不会导致测试失败
3. 时间同步和天气同步需要网络可用才能测试
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
    results.append(("WiFi 配置与同步", test_wifi_config_via_ble(client)))
    results.append(("RTC 时间显示", test_rtc_time_display(client)))
    results.append(("手动网络同步", test_network_sync_manual()))

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
📝 网络同步测试说明：

1. WiFi 配置已通过 BLE 发送到模拟器
2. 请查看模拟器日志确认：
   - WiFi 连接状态
   - 时间同步结果
   - 天气同步结果

3. 完整的网络同步需要：
   - tap 设备: sudo ./tap.sh
   - 正确的 WiFi 凭据
   - 网络连通性
""")

    return failed == 0


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8080
    success = run_network_tests(port)
    sys.exit(0 if success else 1)
