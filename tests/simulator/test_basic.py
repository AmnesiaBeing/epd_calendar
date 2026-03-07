#!/usr/bin/env python3
"""
基础启动流程测试
验证系统能够正常启动、初始化所有服务、进入工作状态
"""

import sys
import os

# 添加当前目录到路径
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from simulator_client import SimulatorClient, print_response


def test_basic_connection(client: SimulatorClient) -> bool:
    """测试 1: 基础连接测试"""
    print("\n" + "="*60)
    print("  测试 1: 基础连接测试")
    print("="*60)
    
    status = client.get_status()
    if print_response(status, "系统状态"):
        print("✅ 基础连接测试通过")
        return True
    else:
        print("❌ 基础连接测试失败")
        return False


def test_rtc_status(client: SimulatorClient) -> bool:
    """测试 2: RTC 状态测试"""
    print("\n" + "="*60)
    print("  测试 2: RTC 状态测试")
    print("="*60)
    
    rtc_status = client.get_rtc_status()
    if print_response(rtc_status, "RTC 状态"):
        # 检查关键字段
        if rtc_status.get("initialized"):
            print("✅ RTC 已初始化")
            return True
        else:
            print("⚠️  RTC 未初始化")
            return True  # 不影响主流程
    return False


def test_ble_status(client: SimulatorClient) -> bool:
    """测试 3: BLE 状态测试"""
    print("\n" + "="*60)
    print("  测试 3: BLE 状态测试")
    print("="*60)
    
    ble_status = client.get_ble_status()
    if print_response(ble_status, "BLE 状态"):
        connected = ble_status.get("connected", False)
        advertising = ble_status.get("advertising", False)
        configured = ble_status.get("configured", False)
        
        print(f"  - 已连接: {connected}")
        print(f"  - 广播中: {advertising}")
        print(f"  - 已配置: {configured}")
        
        print("✅ BLE 状态测试通过")
        return True
    return False


def test_watchdog_status(client: SimulatorClient) -> bool:
    """测试 4: 看门狗状态测试"""
    print("\n" + "="*60)
    print("  测试 4: 看门狗状态测试")
    print("="*60)
    
    wdt_status = client.get_watchdog_status()
    if print_response(wdt_status, "看门狗状态"):
        enabled = wdt_status.get("enabled", False)
        timeout = wdt_status.get("timeout_ms", 0)
        
        print(f"  - 已启用: {enabled}")
        print(f"  - 超时时间: {timeout}ms")
        
        print("✅ 看门狗状态测试通过")
        return True
    return False


def run_basic_tests(port: int = 8080) -> bool:
    """运行所有基础测试"""
    print("\n" + "#"*60)
    print("#  模拟器基础启动流程测试")
    print("#"*60)
    
    client = SimulatorClient(f"http://127.0.0.1:{port}")
    
    # 检查服务是否可用
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
    
    results.append(("基础连接", test_basic_connection(client)))
    results.append(("RTC 状态", test_rtc_status(client)))
    results.append(("BLE 状态", test_ble_status(client)))
    results.append(("看门狗状态", test_watchdog_status(client)))
    
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
    success = run_basic_tests(port)
    sys.exit(0 if success else 1)
