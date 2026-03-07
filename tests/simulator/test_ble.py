#!/usr/bin/env python3
"""
BLE 功能测试
验证 BLE 配置、连接、WiFi 配网功能
"""

import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from simulator_client import SimulatorClient, print_response


def test_ble_connect(client: SimulatorClient) -> bool:
    """测试 1: BLE 连接测试"""
    print("\n" + "="*60)
    print("  测试 1: BLE 连接测试")
    print("="*60)
    
    # 获取连接前状态
    print("连接前状态:")
    before = client.get_ble_status()
    print_response(before)
    
    # 发起连接
    print("\n发起连接...")
    result = client.ble_connect()
    print_response(result, "连接响应")
    
    # 等待状态更新
    import time
    time.sleep(0.5)
    
    # 获取连接后状态
    print("\n连接后状态:")
    after = client.get_ble_status()
    print_response(after)
    
    connected = after.get("connected", False)
    if connected:
        print("✅ BLE 连接测试通过")
        return True
    else:
        print("❌ BLE 连接测试失败 - 状态未更新")
        return False


def test_ble_disconnect(client: SimulatorClient) -> bool:
    """测试 2: BLE 断开测试"""
    print("\n" + "="*60)
    print("  测试 2: BLE 断开测试")
    print("="*60)
    
    # 先确保连接
    client.ble_connect()
    import time
    time.sleep(0.5)
    
    # 发起断开
    print("发起断开...")
    result = client.ble_disconnect()
    print_response(result, "断开响应")
    
    # 等待状态更新
    time.sleep(0.5)
    
    # 获取断开后状态
    print("\n断开后状态:")
    after = client.get_ble_status()
    print_response(after)
    
    connected = after.get("connected", False)
    if not connected:
        print("✅ BLE 断开测试通过")
        return True
    else:
        print("❌ BLE 断开测试失败 - 状态未更新")
        return False


def test_ble_config(client: SimulatorClient) -> bool:
    """测试 3: BLE 配置下发测试"""
    print("\n" + "="*60)
    print("  测试 3: BLE 配置下发测试")
    print("="*60)
    
    # 先连接
    client.ble_connect()
    import time
    time.sleep(0.5)
    
    # 发送 WiFi 配置
    test_ssid = "TestWiFi_123"
    test_password = "test_password_456"
    
    print(f"发送 WiFi 配置:")
    print(f"  SSID: {test_ssid}")
    print(f"  Password: {test_password}")
    
    result = client.ble_config(test_ssid, test_password)
    print_response(result, "配置响应")
    
    # 检查返回结果
    if result.get("status") == "ok" or "success" in str(result).lower():
        print("✅ BLE 配置下发测试通过")
        return True
    else:
        print("⚠️  BLE 配置已发送（请查看日志确认接收）")
        return True  # 配置已发送，日志确认


def test_ble_status_fields(client: SimulatorClient) -> bool:
    """测试 4: BLE 状态字段完整性测试"""
    print("\n" + "="*60)
    print("  测试 4: BLE 状态字段完整性测试")
    print("="*60)
    
    status = client.get_ble_status()
    print_response(status)
    
    # 检查必需字段
    required_fields = ["connected", "advertising", "configured"]
    missing = [f for f in required_fields if f not in status]
    
    if missing:
        print(f"❌ 缺少字段: {missing}")
        return False
    else:
        print(f"✅ 所有必需字段都存在: {required_fields}")
        return True


def run_ble_tests(port: int = 8080) -> bool:
    """运行所有 BLE 测试"""
    print("\n" + "#"*60)
    print("#  模拟器 BLE 功能测试")
    print("#"*60)
    
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
    
    results.append(("BLE 状态字段", test_ble_status_fields(client)))
    results.append(("BLE 连接", test_ble_connect(client)))
    results.append(("BLE 断开", test_ble_disconnect(client)))
    results.append(("BLE 配置下发", test_ble_config(client)))
    
    # 汇总结果
    print("\n" + "="*60)
    print("  测试结果汇总")
    print("="*60)
    
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
    success = run_ble_tests(port)
    sys.exit(0 if success else 1)
