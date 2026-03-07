#!/usr/bin/env python3
"""
综合测试运行器
运行所有模拟器测试用例
"""

import sys
import os
import subprocess
import time
import signal

# 添加当前目录到路径
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from simulator_client import SimulatorClient


class TestRunner:
    """测试运行器"""

    def __init__(self, project_dir: str = None):
        self.project_dir = project_dir or os.path.dirname(
            os.path.dirname(os.path.abspath(__file__))
        )
        self.simulator_process = None
        self.port = 8080

    def compile_simulator(self) -> bool:
        """编译模拟器"""
        print("\n" + "=" * 60)
        print("  编译模拟器")
        print("=" * 60)

        result = subprocess.run(
            ["cargo", "bs"], cwd=self.project_dir, capture_output=True, text=True
        )

        if result.returncode != 0:
            print("❌ 编译失败:")
            print(result.stderr)
            return False

        print("✅ 编译成功")
        return True

    def start_simulator(self, with_graphics: bool = False) -> bool:
        """启动模拟器"""
        print("\n" + "=" * 60)
        print("  启动模拟器")
        print("=" * 60)

        cmd = ["cargo", "rs"]
        if with_graphics:
            cmd = ["cargo", "rsg"]

        self.simulator_process = subprocess.Popen(
            cmd,
            cwd=self.project_dir,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        # 等待服务启动
        print("等待服务启动...")
        for i in range(30):
            try:
                client = SimulatorClient(f"http://127.0.0.1:{self.port}", timeout=2)
                status = client.get_status()
                if "error" not in status:
                    print(f"✅ 模拟器已启动 (端口 {self.port})")
                    return True
            except:
                pass
            time.sleep(1)

        print("❌ 模拟器启动超时")
        return False

    def stop_simulator(self):
        """停止模拟器"""
        if self.simulator_process:
            print("\n停止模拟器...")
            self.simulator_process.terminate()
            try:
                self.simulator_process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.simulator_process.kill()
            print("✅ 模拟器已停止")

    def wait_for_input(self, prompt: str = "按回车键继续...") -> None:
        """等待用户输入"""
        try:
            input(prompt)
        except:
            pass

    def run_test_script(self, script_path: str) -> bool:
        """运行测试脚本"""
        print(f"\n运行测试: {os.path.basename(script_path)}")

        result = subprocess.run(
            [sys.executable, script_path, str(self.port)], cwd=self.project_dir
        )

        return result.returncode == 0


def main():
    """主函数"""
    import argparse

    parser = argparse.ArgumentParser(description="模拟器综合测试")
    parser.add_argument("--no-compile", action="store_true", help="跳过编译")
    parser.add_argument("--graphics", action="store_true", help="使用图形模式")
    parser.add_argument("--port", type=int, default=8080, help="HTTP 服务端口")
    parser.add_argument(
        "tests",
        nargs="*",
        default=["basic", "ble", "button"],
        help="要运行的测试 (basic, ble, button, all)",
    )

    args = parser.parse_args()

    runner = TestRunner()
    runner.port = args.port

    test_map = {
        "basic": "test_basic.py",
        "ble": "test_ble.py",
        "button": "test_button.py",
    }

    try:
        # 编译
        if not args.no_compile:
            if not runner.compile_simulator():
                return 1

        # 启动模拟器
        if not runner.start_simulator(args.graphics):
            return 1

        # 等待用户确认
        print("\n" + "=" * 60)
        print("  模拟器已启动")
        print("=" * 60)
        print(f"请在另一个终端查看日志: cargo rs")
        runner.wait_for_input("按回车键开始测试...")

        # 运行测试
        all_passed = True
        tests_to_run = args.tests

        if "all" in tests_to_run:
            tests_to_run = list(test_map.keys())

        for test_name in tests_to_run:
            if test_name in test_map:
                script_path = os.path.join(
                    os.path.dirname(__file__), test_map[test_name]
                )
                if not runner.run_test_script(script_path):
                    all_passed = False

        # 汇总
        print("\n" + "#" * 60)
        if all_passed:
            print("#  所有测试完成")
        else:
            print("#  部分测试失败")
        print("#" * 60)

        return 0 if all_passed else 1

    finally:
        runner.stop_simulator()


if __name__ == "__main__":
    sys.exit(main())
