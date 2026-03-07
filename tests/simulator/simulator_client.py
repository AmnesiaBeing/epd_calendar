#!/usr/bin/env python3
"""
模拟器 HTTP API 客户端
用于与模拟器 HTTP 服务进行交互测试
"""

import requests
import json
import time
from typing import Dict, Any, Optional


class SimulatorClient:
    """模拟器 HTTP API 客户端"""

    def __init__(self, base_url: str = "http://127.0.0.1:8080", timeout: int = 10):
        self.base_url = base_url
        self.timeout = timeout
        self.session = requests.Session()

    def _request(self, method: str, endpoint: str, **kwargs) -> Dict[str, Any]:
        """发送 HTTP 请求"""
        url = f"{self.base_url}{endpoint}"
        try:
            response = self.session.request(method, url, timeout=self.timeout, **kwargs)
            response.raise_for_status()
            return response.json()
        except requests.exceptions.RequestException as e:
            return {"error": str(e), "status": "failed"}

    def get_status(self) -> Dict[str, Any]:
        """获取系统整体状态"""
        return self._request("GET", "/status")

    def get_rtc_status(self) -> Dict[str, Any]:
        """获取 RTC 状态"""
        return self._request("GET", "/status/rtc")

    def get_ble_status(self) -> Dict[str, Any]:
        """获取 BLE 状态"""
        return self._request("GET", "/status/ble")

    def get_watchdog_status(self) -> Dict[str, Any]:
        """获取看门狗状态"""
        return self._request("GET", "/status/watchdog")

    def press_button(self, button_type: str) -> Dict[str, Any]:
        """
        模拟按键事件

        Args:
            button_type: 按键类型 (short_press, double_click, triple_click, long_press)
        """
        event_map = {
            "short": "short_press",
            "long": "long_press",
            "double": "double_click",
            "triple": "triple_click",
        }
        event = event_map.get(button_type, button_type)
        return self._request("POST", "/api/button", json={"event": event})

    def ble_connect(self) -> Dict[str, Any]:
        """模拟 BLE 连接"""
        return self._request("POST", "/api/ble/connect")

    def ble_disconnect(self) -> Dict[str, Any]:
        """模拟 BLE 断开"""
        return self._request("POST", "/api/ble/disconnect")

    def ble_config(self, ssid: str, password: str) -> Dict[str, Any]:
        """
        模拟 BLE 配置下发

        Args:
            ssid: WiFi 名称
            password: WiFi 密码
        """
        return self._request(
            "POST",
            "/api/ble/config",
            json={"data": {"ssid": ssid, "password": password}},
        )

    def wait_for_status(
        self, key: str, expected_value: Any, timeout: int = 10, interval: float = 0.5
    ) -> bool:
        """
        等待状态变化

        Args:
            key: 要检查的状态键
            expected_value: 期望的值
            timeout: 超时时间（秒）
            interval: 检查间隔（秒）

        Returns:
            bool: 是否达到期望值
        """
        start_time = time.time()
        while time.time() - start_time < timeout:
            status = self.get_status()
            if key in status and status[key] == expected_value:
                return True
            time.sleep(interval)
        return False


def print_response(response: Dict[str, Any], title: str = "") -> bool:
    """格式化打印响应"""
    if title:
        print(f"\n{'=' * 60}")
        print(f"  {title}")
        print(f"{'=' * 60}")

    if "error" in response:
        print(f"❌ 错误: {response['error']}")
        return False

    print(json.dumps(response, indent=2, ensure_ascii=False))
    return True


if __name__ == "__main__":
    # 测试客户端
    client = SimulatorClient()

    print("🔍 测试连接...")
    status = client.get_status()
    print_response(status, "系统状态")
