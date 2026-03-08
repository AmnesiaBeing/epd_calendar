#!/usr/bin/env python3
"""
配置测试综合运行脚本
按顺序运行所有配置相关测试
"""

import sys
import os
import subprocess
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def run_test(script_name: str, description: str) -> bool:
    """运行单个测试脚本"""
    print("\n" + "#" * 60)
    print(f"#  {description}")
    print("#" * 60)

    script_path = os.path.join(os.path.dirname(__file__), script_name)

    if not os.path.exists(script_path):
        print(f"❌ 测试脚本不存在: {script_path}")
        return False

    result = subprocess.run([sys.executable, script_path])

    if result.returncode == 0:
        print(f"✅ {description} 通过")
        return True
    else:
        print(f"❌ {description} 失败")
        return False


def main():
    """主函数"""
    print("\n" + "=" * 60)
    print("  ConfigManager 配置测试套件")
    print("=" * 60)

    print("\n⚠️  重要提示:")
    print("  1. 请确保模拟器已启动: cargo rs")
    print("  2. 测试将按顺序执行，请勿并行运行")
    print("  3. Flash 文件位置: /tmp/simulator_flash.bin")

    input("\n按回车键开始测试...")

    # 测试列表
    tests = [
        ("test_config_persistence.py", "配置持久化测试"),
        ("test_config_update.py", "配置更新测试"),
        ("test_config_integrity.py", "配置完整性测试"),
    ]

    # 运行测试
    results = []
    for script, desc in tests:
        result = run_test(script, desc)
        results.append((desc, result))

        # 测试间隔
        if script != tests[-1][0]:
            print("\n等待 2 秒...")
            time.sleep(2)

    # 汇总结果
    print("\n" + "=" * 60)
    print("  测试结果汇总")
    print("=" * 60)

    passed = sum(1 for _, r in results if r)
    failed = len(results) - passed

    for desc, result in results:
        status = "✅ 通过" if result else "❌ 失败"
        print(f"  {desc}: {status}")

    print(f"\n总计: {passed}/{len(results)} 通过")

    if failed > 0:
        print(f"\n❌ {failed} 个测试失败")
        return 1
    else:
        print("\n✅ 所有测试通过")
        return 0


if __name__ == "__main__":
    sys.exit(main())
