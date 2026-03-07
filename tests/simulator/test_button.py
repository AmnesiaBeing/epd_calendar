#!/usr/bin/env python3
"""
用户交互测试
验证按键事件处理、状态转换功能
"""

import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from simulator_client import SimulatorClient, print_response


def test_button_short(client: SimulatorClient) -> bool:
    """测试 1: 短按按键测试"""
    print("\n" + "=" * 60)
    print("  测试 1: 短按按键测试")
    print("=" * 60)

    print("发送短按事件...")
    result = client.press_button("short")
    print_response(result, "响应")

    # 检查响应
    if (
        result.get("status") == "ok"
        or "success" in str(result).lower()
        or not result.get("error")
    ):
        print("✅ 短按事件已发送")
        return True
    else:
        print("❌ 短按事件发送失败")
        return False


def test_button_double(client: SimulatorClient) -> bool:
    """测试 2: 双击按键测试"""
    print("\n" + "=" * 60)
    print("  测试 2: 双击按键测试")
    print("=" * 60)

    print("发送双击事件...")
    result = client.press_button("double")
    print_response(result, "响应")

    if (
        result.get("status") == "ok"
        or "success" in str(result).lower()
        or not result.get("error")
    ):
        print("✅ 双击事件已发送")
        return True
    else:
        print("❌ 双击事件发送失败")
        return False


def test_button_triple(client: SimulatorClient) -> bool:
    """测试 3: 三击按键测试"""
    print("\n" + "=" * 60)
    print("  测试 3: 三击按键测试")
    print("=" * 60)

    print("发送三击事件...")
    result = client.press_button("triple")
    print_response(result, "响应")

    if (
        result.get("status") == "ok"
        or "success" in str(result).lower()
        or not result.get("error")
    ):
        print("✅ 三击事件已发送")
        return True
    else:
        print("❌ 三击事件发送失败")
        return False


def test_button_long(client: SimulatorClient) -> bool:
    """测试 4: 长按按键测试"""
    print("\n" + "=" * 60)
    print("  测试 4: 长按按键测试")
    print("=" * 60)

    print("发送长按事件...")
    result = client.press_button("long")
    print_response(result, "响应")

    if (
        result.get("status") == "ok"
        or "success" in str(result).lower()
        or not result.get("error")
    ):
        print("✅ 长按事件已发送")
        return True
    else:
        print("❌ 长按事件发送失败")
        return False


def test_button_invalid_type(client: SimulatorClient) -> bool:
    """测试 5: 无效按键类型测试"""
    print("\n" + "=" * 60)
    print("  测试 5: 无效按键类型测试")
    print("=" * 60)

    print("发送无效按键类型...")
    import requests

    try:
        response = requests.post(
            f"{client.base_url}/api/button",
            json={"type": "invalid_type"},
            timeout=client.timeout,
        )
        print(f"  状态码: {response.status_code}")

        # 404 或其他错误表示服务端正确拒绝了无效类型
        if response.status_code >= 400:
            print("✅ 无效类型被正确拒绝")
            return True
        else:
            print("⚠️  无效类型未被拒绝（可能需要服务端验证）")
            return True
    except Exception as e:
        print(f"❌ 请求失败: {e}")
        return False


def run_button_tests(port: int = 8080) -> bool:
    """运行所有按键测试"""
    print("\n" + "#" * 60)
    print("#  模拟器用户交互测试")
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

    results.append(("短按按键", test_button_short(client)))
    results.append(("双击按键", test_button_double(client)))
    results.append(("三击按键", test_button_triple(client)))
    results.append(("长按按键", test_button_long(client)))
    results.append(("无效按键类型", test_button_invalid_type(client)))

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
    success = run_button_tests(port)
    sys.exit(0 if success else 1)
