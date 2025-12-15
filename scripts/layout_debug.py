import yaml
import re
from typing import Dict, List, Any, Union
import os

# 内置字体大小映射（支持扩展）
FONT_SIZE_MAPPING = {
    "Small": 16,
    "Medium": 24,
    "Large": 40,
    # 可扩展新增
    # "XLarge": 48,
    # "XSmall": 12
}

# 规则配置
RULES = {
    "max_lengths": {
        "id": 32,
        "content": 128,
        "condition": 128,
        "icon_id": 64
    },
    "allowed_node_types": ["container", "text", "icon", "line"],
    "allowed_icon_modules": ["digit_icon", "weather_icon", "system_icon"],
    "forbidden_operators": ["+", "-", "*", "/", "%", "?"],  # 仅花括号内禁用
    "max_nest_level": 2,  # 表达式嵌套层级
    "max_node_nest": 10,   # 节点嵌套层级
    "max_children": 20,   # container子节点最大数量
    # 节点属性规则（必选/可选/禁止）
    "node_attributes": {
        "container": {
            "required": ["id", "type", "position", "anchor"],
            "optional": ["direction", "alignment", "vertical_alignment", "children", "weight", "condition", "width", "height"],
            "forbidden": ["start", "end", "thickness", "is_absolute", "icon_id", "content", "font_size", "max_width", "max_lines"]
        },
        "text": {
            "required": ["id", "type", "position", "anchor", "content"],
            "optional": ["font_size", "alignment", "vertical_alignment", "max_width", "max_lines", "weight", "is_absolute", "condition", "width", "height"],
            "forbidden": ["start", "end", "thickness", "icon_id"]
        },
        "icon": {
            "required": ["id", "type", "position", "anchor", "icon_id"],
            "optional": ["alignment", "vertical_alignment", "weight", "is_absolute", "condition", "width", "height"],
            "forbidden": ["start", "end", "thickness", "content", "font_size", "max_width", "max_lines"]
        },
        "line": {
            "required": ["id", "type", "start", "end", "thickness"],
            "optional": ["is_absolute", "condition"],
            "forbidden": ["position", "anchor", "width", "height", "icon_id", "content", "font_size", "max_width", "max_lines", "weight", "direction", "alignment", "vertical_alignment"]
        }
    },
    # 属性取值约束
    "attribute_constraints": {
        "anchor": {"allowed_values": ["top-left", "top-center", "top-right", "center-left", "center", "center-right", "bottom-left", "bottom-center", "bottom-right"]},
        "font_size": {"allowed_types": [int, float, str], "min": 8, "max": 64},  # 字体大小范围8-64px
        "alignment": {"allowed_values": ["center", "left", "right"]},
        "vertical_alignment": {"allowed_values": ["center", "top", "bottom"]},
        "direction": {"allowed_values": ["horizontal", "vertical"]},
        "max_width": {"min": 0, "max": 800, "type": int},
        "max_lines": {"min": 1, "max": 5, "type": int},
        "weight": {"min": 0.0001, "max": 10.0, "type": (int, float)},
        "thickness": {"min": 1, "max": 3, "type": int},
        "is_absolute": {"type": bool},
        "width": {"min": 0, "type": (int, float)},
        "height": {"min": 0, "type": (int, float)},
        # 屏幕尺寸800x480
        "coordinate": {"x": (0, 800), "y": (0, 480)}
    },
    # 绝对布局允许的节点类型
    "is_absolute_allowed": ["text", "icon", "line"],
    # weight使用约束：父容器必须是container且有direction
    "weight_parent_require": ["container", "direction"]
}

def extract_brace_content(text: str) -> List[str]:
    """提取所有花括号 {} 内部的内容（排除转义的花括号）"""
    brace_contents = []
    pattern = re.compile(r'(?<!\\){(.*?)(?<!\\)}', re.DOTALL)
    matches = pattern.findall(text)
    for match in matches:
        cleaned = re.sub(r'\\([\-+\\*/%?])', r'\1', match)
        brace_contents.append(cleaned)
    return brace_contents

def get_expr_nest_level(expr: str) -> int:
    """计算表达式中{}的嵌套层级（仅统计未转义的花括号）"""
    level = 0
    max_level = 0
    escaped = False
    for char in expr:
        if escaped:
            escaped = False
            continue
        if char == "\\":
            escaped = True
            continue
        if char == "{":
            level += 1
            max_level = max(max_level, level)
        elif char == "}":
            level -= 1
    return max_level

def parse_font_size(value: Union[int, float, str]) -> Union[int, float, None]:
    """解析字体大小值（支持数字/字符串）"""
    if isinstance(value, (int, float)):
        return value
    elif isinstance(value, str):
        # 处理带px的字符串（如"16px"）
        if value.endswith("px"):
            try:
                return int(value.replace("px", ""))
            except ValueError:
                return None
        # 处理内置别名（如"Small"）
        return FONT_SIZE_MAPPING.get(value, None)
    return None

def validate_coordinate(coord: list, coord_name: str, is_absolute: bool, path: str) -> List[str]:
    """校验坐标（start/end/position）合法性"""
    errors = []
    # 类型校验
    if not isinstance(coord, list):
        errors.append(f"[{path}.{coord_name}] 类型错误：需为数组，实际为{type(coord).__name__}")
        return errors
    # 长度校验
    if len(coord) != 2:
        errors.append(f"[{path}.{coord_name}] 格式错误：需为二元数组 [x,y]，实际为{coord}")
    # 数值校验
    for idx, (val, dim) in enumerate(zip(coord, ["x", "y"])):
        if not isinstance(val, (int, float)):
            errors.append(f"[{path}.{coord_name}.{dim}] 类型错误：需为数字，实际为{type(val).__name__}")
        if val < 0:
            errors.append(f"[{path}.{coord_name}.{dim}] 取值错误：需≥0，实际为{val}")
    # 绝对布局范围校验
    if is_absolute and len(coord) == 2:
        x, y = coord[:2]
        x_range = RULES["attribute_constraints"]["coordinate"]["x"]
        y_range = RULES["attribute_constraints"]["coordinate"]["y"]
        if x < x_range[0] or x > x_range[1]:
            errors.append(f"[{path}.{coord_name}.x] 绝对坐标超限：需{x_range[0]}≤x≤{x_range[1]}，实际为{x}")
        if y < y_range[0] or y > y_range[1]:
            errors.append(f"[{path}.{coord_name}.y] 绝对坐标超限：需{y_range[0]}≤y≤{y_range[1]}，实际为{y}")
    return errors

def validate_attribute_constraints(node: Dict[str, Any], node_type: str, path: str) -> List[str]:
    """校验属性取值范围"""
    errors = []
    is_absolute = node.get("is_absolute", False)
    
    # 校验坐标类属性
    if node_type == "line":
        for coord_attr in ["start", "end"]:
            if coord_attr in node:
                errors.extend(validate_coordinate(node[coord_attr], coord_attr, is_absolute, path))
    elif node_type in ["container", "text", "icon"]:
        if "position" in node:
            errors.extend(validate_coordinate(node["position"], "position", is_absolute, path))
    
    # 校验枚举类属性（anchor/alignment/vertical_alignment/direction）
    for attr, constraints in RULES["attribute_constraints"].items():
        if attr not in node or "allowed_values" not in constraints:
            continue
        value = node[attr]
        if value not in constraints["allowed_values"]:
            errors.append(f"[{path}.{attr}] 取值错误：需为{','.join(constraints['allowed_values'])}，实际为{value}")
    
    # 校验字体大小
    if "font_size" in node:
        fs_value = node["font_size"]
        parsed_fs = parse_font_size(fs_value)
        if parsed_fs is None:
            errors.append(f"[{path}.font_size] 格式错误：不支持的字体大小 '{fs_value}'，支持数字/px后缀/内置别名（{','.join(FONT_SIZE_MAPPING.keys())}）")
        else:
            fs_min = RULES["attribute_constraints"]["font_size"]["min"]
            fs_max = RULES["attribute_constraints"]["font_size"]["max"]
            if parsed_fs < fs_min or parsed_fs > fs_max:
                errors.append(f"[{path}.font_size] 取值超限：需在{fs_min}~{fs_max}px之间，实际为{parsed_fs}px")
    
    # 校验数值范围类属性（max_width/max_lines/weight/thickness/width/height）
    for attr, constraints in RULES["attribute_constraints"].items():
        if attr not in node or "min" not in constraints:
            continue
        if attr == "font_size":
            continue  # 单独处理
        
        value = node[attr]
        # 类型校验
        if not isinstance(value, constraints["type"]):
            type_name = constraints["type"].__name__ if isinstance(constraints["type"], type) else "数字"
            errors.append(f"[{path}.{attr}] 类型错误：需为{type_name}，实际为{type(value).__name__}")
            continue
        
        # 范围校验
        if "max" in constraints and (value < constraints["min"] or value > constraints["max"]):
            errors.append(f"[{path}.{attr}] 取值超限：需在{constraints['min']}~{constraints['max']}之间，实际为{value}")
        elif value < constraints["min"]:
            errors.append(f"[{path}.{attr}] 取值错误：需≥{constraints['min']}，实际为{value}")
    
    # 校验is_absolute使用场景
    if "is_absolute" in node and node_type not in RULES["is_absolute_allowed"]:
        errors.append(f"[{path}.is_absolute] 非法使用：仅{','.join(RULES['is_absolute_allowed'])}节点允许绝对布局")
    
    return errors

def validate_weight_usage(node: Dict[str, Any], parent_node: Dict[str, Any], path: str) -> List[str]:
    """校验weight使用约束"""
    errors = []
    if "weight" not in node:
        return errors
    # 父容器必须是container且有direction
    if parent_node.get("type") != RULES["weight_parent_require"][0] or RULES["weight_parent_require"][1] not in parent_node:
        errors.append(f"[{path}.weight] 非法使用：仅父容器为container且指定direction时可用")
    return errors

def validate_node_required_attrs(node: Dict[str, Any], node_type: str, path: str) -> List[str]:
    """校验节点必选属性"""
    errors = []
    if not node_type:
        return errors
    required_attrs = RULES["node_attributes"][node_type]["required"]
    for attr in required_attrs:
        if attr not in node:
            errors.append(f"[{path}] 缺少必选属性：{attr}（{node_type}节点必填）")
    return errors

def validate_node_forbidden_attrs(node: Dict[str, Any], node_type: str, path: str) -> List[str]:
    """校验节点禁止属性"""
    errors = []
    if not node_type:
        return errors
    forbidden_attrs = RULES["node_attributes"][node_type]["forbidden"]
    for attr in node.keys():
        if attr in forbidden_attrs and attr != "node":  # node是子节点包装字段，跳过
            errors.append(f"[{path}] 非法属性：{attr}（{node_type}节点禁止使用）")
    return errors

def validate_node_nest_level(node: Dict[str, Any], current_level: int, path: str) -> List[str]:
    """校验节点嵌套层级"""
    errors = []
    if current_level > RULES["max_node_nest"]:
        errors.append(f"[{path}] 嵌套层级超限：最大允许{str(RULES['max_node_nest'])}层，当前{str(current_level)}层")
        return errors
    # 校验children数量
    if "children" in node and len(node["children"]) > RULES["max_children"]:
        errors.append(f"[{path}.children] 数量超限：最大允许{str(RULES['max_children'])}个子节点，实际{str(len(node['children']))}个")
    # 递归校验子节点
    if "children" in node:
        for idx, child in enumerate(node["children"]):
            child_node = child.get("node", {})
            child_path = f"{path}.children[{idx}]"
            errors.extend(validate_node_nest_level(child_node, current_level + 1, child_path))
    return errors

def validate_expression(node: Dict[str, Any], path: str) -> List[str]:
    """校验表达式（content/icon_id/condition）"""
    errors = []
    for expr_field in ["content", "icon_id", "condition"]:
        if expr_field not in node:
            continue
        raw_expr = str(node[expr_field])
        # 空值校验
        if expr_field in ["content", "icon_id"] and raw_expr.strip() == "":
            errors.append(f"[{path}.{expr_field}] 不能为空")
        # 表达式嵌套层级
        nest_level = get_expr_nest_level(raw_expr)
        if nest_level > RULES["max_nest_level"]:
            errors.append(f"[{path}.{expr_field}] 表达式嵌套超限：最大允许{str(RULES['max_nest_level'])}层，实际{str(nest_level)}层")
        # 花括号内禁用运算符
        brace_contents = extract_brace_content(raw_expr)
        for inner_expr in brace_contents:
            for op in RULES["forbidden_operators"]:
                if op in inner_expr:
                    errors.append(f"[{path}.{expr_field}] 花括号内禁用运算符 '{op}'：{inner_expr}")
        # 图标模块名校验
        if expr_field == "icon_id":
            module_matches = re.findall(r'(?<!\\){([a-zA-Z_]+):', raw_expr)
            for module in module_matches:
                if module not in RULES["allowed_icon_modules"]:
                    errors.append(f"[{path}.icon_id] 非法图标模块：{module}（仅允许{','.join(RULES['allowed_icon_modules'])}）")
    return errors

def validate_node(node: Dict[str, Any], path: str = "root", parent_node: Dict[str, Any] = {}, current_nest: int = 1) -> List[str]:
    """递归校验单个节点"""
    errors = []
    node_type = node.get("type", "")
    
    # 1. 节点类型校验
    if node_type and node_type not in RULES["allowed_node_types"]:
        errors.append(f"[{path}] 非法节点类型：{node_type}（仅允许{','.join(RULES['allowed_node_types'])}）")
    
    # 2. 字段长度校验
    for field, max_len in RULES["max_lengths"].items():
        if field in node:
            value = str(node[field])
            if len(value) > max_len:
                errors.append(f"[{path}.{field}] 长度超限：实际{str(len(value))}字符，最大{str(max_len)}字符")
    
    # 3. 必选属性校验
    errors.extend(validate_node_required_attrs(node, node_type, path))
    
    # 4. 禁止属性校验
    errors.extend(validate_node_forbidden_attrs(node, node_type, path))
    
    # 5. 属性取值约束校验
    errors.extend(validate_attribute_constraints(node, node_type, path))
    
    # 6. weight使用校验
    errors.extend(validate_weight_usage(node, parent_node, path))
    
    # 7. 表达式校验
    errors.extend(validate_expression(node, path))
    
    # 8. 节点嵌套层级校验
    errors.extend(validate_node_nest_level(node, current_nest, path))
    
    # 9. 递归校验子节点
    if "children" in node:
        for idx, child in enumerate(node["children"]):
            child_node = child.get("node", {})
            child_path = f"{path}.children[{idx}]"
            errors.extend(validate_node(child_node, child_path, parent_node=node, current_nest=current_nest + 1))
    
    return errors

def validate_epd_yaml(yaml_content: str) -> List[str]:
    """主校验函数"""
    try:
        data = yaml.safe_load(yaml_content)
    except yaml.YAMLError as e:
        return [f"YAML语法错误：{str(e)}"]
    # 根节点校验
    root_errors = validate_node(data, "root_container")
    return root_errors

if __name__ == "__main__":
    # YAML文件路径
    YAML_FILE_PATH = "./assets/layout/main.yaml"
    
    # 检查文件是否存在
    if not os.path.exists(YAML_FILE_PATH):
        print(f"❌ 错误：文件不存在 → {YAML_FILE_PATH}")
        exit(1)
    
    # 读取YAML文件
    try:
        with open(YAML_FILE_PATH, "r", encoding="utf-8") as f:
            yaml_content = f.read()
    except Exception as e:
        print(f"❌ 读取文件失败：{str(e)}")
        exit(1)
    
    # 执行校验
    errors = validate_epd_yaml(yaml_content)
    
    # 输出结果
    if errors:
        print("❌ 检测到以下违规问题：")
        for idx, error in enumerate(errors, 1):
            print(f"{idx}. {error}")
    else:
        print("✅ YAML完全符合规范")