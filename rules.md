# epd-calendar项目布局规则

基于以下规则提供代码补全、语法校验、错误提示、数据结构联想等功能，确保布局文件（YAML 格式）符合嵌入式系统内存约束、运行时表达式语法规范及数据结构定义，最终可被 `builder` 模块（如 `font_generator.rs`、`icon_generator.rs`）正确解析编译，在运行时可被 `render` 模块正确渲染。

## 1. 字符串长度限制规范

| 字段类型   | 最大长度 | 约束说明                                                                          |
| ------------ | ---------- | ----------------------------------------------------------------------------------- |
| `id`           | 32字符   | 节点唯一标识符，仅允许小写字母（a-z）、数字（0-9）、下划线（_），首字符不能为数字 |
| `content`           | 128字符  | 支持文本内容、图标ID或表达式，超出部分按UTF-8字节截断（避免乱码）                 |
| `condition`           | 128字符  | 条件表达式（支持复杂逻辑），超出长度视为无效表达式                                |
| 模板变量名 | 64字符   | 数据源变量路径（如 `object.property`），使用点号分隔多级结构                                       |
| 字体名称   | 32字符   | 需与 `builder/modules/font_generator.rs` 中定义的字体标识一致                                                        |
| 图标ID     | 64字符   | 格式为 `{图标模块}:{图标键}`（如 `weather_icon:sunny`），需与 `builder/modules/icon_generator.rs` 联动                                                        |

### 长度限制补充说明

1. 限制依据：嵌入式系统内存资源有限，避免长字符串占用过多RAM/ROM
2. 截断策略：严格按UTF-8字节截取，确保截断后字符编码完整（无乱码）
3. 运行时校验：布局解析阶段自动检查字段长度，超长字段记录警告日志（不阻断编译）

## 2. 运行时表达式语法规范

### 2.1 基本语法规则（必须遵循）

```yaml
# 1. 变量引用：强制用花括号 {} 包裹，无嵌套限制
content: "{datetime.hour}"  # 正确
content: "datetime.hour"    # 错误（无花括号）

# 2. 多级属性访问：支持对象嵌套和数组索引（索引从0开始）
content: "{system.config.temperature_unit}"  # 对象多级访问
content: "{weather.daily_weather[0].weather_desc}"  # 数组+对象混合访问
content: "{array[2].property.subproperty}"  # 多级数组+对象

# 3. 字符串拼接：常量文本与变量直接拼接，无需额外符号
content: "当前温度：{sensor.temperature}℃"  # 正确
content: "{datetime.year}年{datetime.month}月{datetime.day}日"  # 多变量拼接
```

### 2.2 表达式运算符（仅在 {} 内生效）

#### 2.2.1 算术运算符（支持四则运算及取模）

```yaml
# 基础用法：+ - * / %
content: "digit_icon:digit_{{datetime.hour}%10}"  # 取小时个位数
content: "第{{index}+1}页"  # 索引从1开始显示
content: "温度翻倍：{{sensor.temperature}*2}℃"

# 运算优先级（与C语言一致，可通过括号改变优先级）
# 1. 括号 () → 最高
# 2. 取负 - → 次之
# 3. * / % → 再次之
# 4. + - → 最低
content: "{{{a}+{b}}*{c}}"  # 先算a+b，再乘c
content: "{-{value}}"  # 取负值
```

#### 2.2.2 比较运算符（数值/字符串专用）

```yaml
# 数值比较（支持整数、浮点数）
condition: "{sensor.temperature} > 25.0"  # 浮点数比较
condition: "{system.battery_level} <= 20"  # 整数比较
condition: "{array.length} != 0"  # 长度比较

# 字符串比较（需用双引号包裹字符串常量）
condition: '{system.network_status} == "connected"'  # 正确
condition: "{system.network_status} == connected"    # 错误（无引号）
condition: '{hitokoto.content} != ""'  # 空字符串检查

# 类型安全约束：数值与字符串直接比较时，表达式视为无效（返回false）
condition: "{sensor.temperature} == '25'"  # 无效表达式（数值≠字符串）
```

#### 2.2.3 逻辑运算符（支持短路求值）

```yaml
# 基础逻辑：&&（与）、||（或）、!（非）
condition: "{system.battery_level} > 20 && {system.is_charging}"  # 电量>20且充电中
condition: "{system.network_status} == 'connected' || {system.network_status} == 'connecting'"  # 网络连接中或已连接
condition: "!{system.config.is_12_hour}"  # 非12小时制

# 短路求值特性：&& 前半为false时，后半不执行；|| 前半为true时，后半不执行
condition: "{array?} && {array[0].valid}"  # 先检查数组存在，再访问元素
condition: "{optional_field} || '默认值'"  # 可选字段为空时用默认值
```

#### 2.2.4 成员运算符（字段属性访问）

```yaml
# 长度获取：支持数组、字符串长度
condition: "{weather.daily_weather.length} == 3"  # 数组长度检查
condition: '{hitokoto.content.length} <= 100'  # 字符串长度限制

# 存在性检查：? 运算符判断字段是否存在（可选字段专用）
condition: "{system.config.wifi.ssid?}"  # 检查wifi ssid是否配置
condition: "{lunar.term?}"  # 检查是否为节气
```

### 2.3 特殊语法（嵌入式场景专用）

#### 2.3.1 图标映射语法

```yaml
# 固定格式：{图标模块名}:{图标键}（模块名需在图标映射规范中定义）
content: "{weather_icon:sunny}"  # 天气图标（模块：weather_icon，键：sunny）
content: "{digit_icon:digit_8}"  # 数字图标（模块：digit_icon，键：digit_8）

# 动态键值：图标键支持表达式（需嵌套 {}）
content: "{weather_icon:{weather.daily_weather[0].weather_icon}}"  # 动态获取天气图标
content: "{system_icon:network_{system.network_status}}"  # 动态获取网络状态图标
```

#### 2.3.2 三元条件表达式

```yaml
# 格式：{条件 ? 真值 : 假值}（支持嵌套）
content: "{{system.is_charging} ? '充电中' : '未充电'}"  # 文本动态切换
content: "{{sensor.temp_valid} ? {sensor.temperature}℃ : '无数据'}"  # 数据有效性判断
condition: "{{system.battery_level} > 50 ? {system.is_charging} : false}"  # 条件嵌套
```

### 2.4 表达式执行上下文与错误处理

#### 2.4.1 变量作用域

- 仅支持访问全局数据源（如 `datetime`、`weather`、`sensor` 等，见数据结构定义）
- 不支持自定义局部变量，变量名需与数据源字段完全一致（大小写敏感）

#### 2.4.2 错误处理规则

1. 变量不存在：返回空字符串（`content` 字段）或 false（`condition` 字段）
2. 类型错误（如数值运算中含字符串）：尝试自动转换，失败则返回默认值（空字符串/false）
3. 除零错误：打印警告日志，返回系统最大值（数值类型）或空字符串（文本类型）
4. 表达式语法错误：视为无效表达式，`content` 显示空字符串，`condition` 返回 false

## 3. 标准数据结构定义（全局数据源）

### 3.1 时间数据结构（`datetime`）

```json
{
  "datetime": {
    "hour": 14,           // 24小时制（0-23），整数
    "minute": 30,         // 0-59，整数
    "year": 2023,         // 完整年份（4位），整数
    "month": 12,          // 1-12，整数
    "day": 31,            // 1-31，整数
    "weekday": 0,         // 0=周日，1=周一，…，6=周六，整数
    "holiday": "元旦",    // 节假日名称（空字符串=非节假日），字符串
    "lunar": {            // 农历嵌套对象
      "ganzhi": 34,       // 干支序号（1-60），整数
      "zodiac": 1,        // 生肖序号（1-12），整数
      "month": 6,         // 农历月份（1-12，负数=闰月），整数
      "day": 23,          // 农历日期（1-30），整数
      "term": "冬至",      // 节气名称（空字符串=非节气），字符串
      "festival": "",     // 农历节日（空字符串=非节日），字符串
      "recommends": ["祭祀", "祈福"], // 宜做事项（每项≤8字符），字符串数组
      "avoids": ["嫁娶", "动土"],     // 忌做事项（每项≤8字符），字符串数组
    }
  }
}
```

### 3.2 天气数据结构（`weather`）

```json
{
  "weather": {
    "valid": true,             // 数据有效性，布尔值
    "location": "北京市海淀区",  // 位置信息，字符串
    "daily_weather": [         // 固定3天预报（今天、明天、后天），数组
      {
        "date": "2023-12-31",  // 日期字符串（YYYY-MM-DD）
        "weather_desc": "晴",  // 天气描述，字符串
        "weather_icon": "sunny", // 天气图标键（映射到weather_icon模块）
        "weather_code": 0,     // 天气编码，整数
        "temp_high": 28,       // 最高温度（整数）
        "temp_low": 18         // 最低温度（整数）
      },
      {
        "date": "2024-01-01",
        "weather_desc": "多云",
        "weather_icon": "cloudy",
        "weather_code": 1,
        "temp_high": 26,
        "temp_low": 16
      },
      {
        "date": "2024-01-02",
        "weather_desc": "小雨",
        "weather_icon": "rainy",
        "weather_code": 3,
        "temp_high": 24,
        "temp_low": 15
      }
    ]
  }
}
```

### 3.3 传感器数据结构（`sensor`）

```json
{
  "sensor": {
    "temperature": 23.5,  // 当前温度（浮点数）
    "temp_valid": true,   // 温度数据有效性，布尔值
    "humidity": 55.0,     // 当前湿度（0-100，浮点数）
    "humi_valid": true    // 湿度数据有效性，布尔值
  }
}
```

### 3.4 格言数据结构（`hitokoto`）

```json
{
  "hitokoto": {
    "content": "天行健，君子以自强不息",  // 格言正文（必选），字符串
    "from": "周易",                     // 出处（必选），字符串
    "from_who": ""                      // 作者（空字符串=未知），字符串
  }
}
```

### 3.5 系统状态数据结构（`system`）

```json
{
  "system": {
    "battery_level": 85,           // 电量百分比（0-100），整数
    "battery_voltage": 3.8,        // 电池电压（V），浮点数
    "is_charging": true,           // 是否充电，布尔值
    "network_status": "connected",  // 网络状态（仅支持connected/disconnected/connecting）
    "ip_address": "192.168.1.100", // IP地址（空字符串=未连接），字符串
    "config": {
      "is_12_hour": false,          // 是否12小时制，布尔值
      "is_am": true,                // 上午/下午（仅12小时制有效），布尔值
      "temperature_unit": "celsius", // 温度单位（仅支持celsius/fahrenheit）
      "wifi.ssid": "",              // WiFi名称，字符串
      "wifi.password": ""           // WiFi密码，字符串
    }
  }
}
```

### 3.6 图标映射规范（`icon` 模块定义）

```json
{
  "digit_icon": {  // 数字图标模块（time_digit）
    "digit_0": "assets/icons/time_digit/digit_0.svg",
    "digit_1": "assets/icons/time_digit/digit_1.svg",
    "digit_2": "assets/icons/time_digit/digit_2.svg",
    "digit_3": "assets/icons/time_digit/digit_3.svg",
    "digit_4": "assets/icons/time_digit/digit_4.svg",
    "digit_5": "assets/icons/time_digit/digit_5.svg",
    "digit_6": "assets/icons/time_digit/digit_6.svg",
    "digit_7": "assets/icons/time_digit/digit_7.svg",
    "digit_8": "assets/icons/time_digit/digit_8.svg",
    "digit_9": "assets/icons/time_digit/digit_9.svg",
    "digit_colon": "assets/icons/time_digit/digit_colon.svg"
  },
  "weather_icon": {  // 天气图标模块（weather）
    // 以和风天气图标为准
  },
  "system_icon": {  // 系统图标模块（system）
    "network_connected": "assets/icons/network/connected.svg",
    "network_disconnected": "assets/icons/network/disconnected.svg",
    "network_connecting": "assets/icons/network/connecting.svg",
    "battery_0": "assets/icons/battery/battery_0.svg",
    "battery_1": "assets/icons/battery/battery_1.svg",
    "battery_2": "assets/icons/battery/battery_2.svg",
    "battery_3": "assets/icons/battery/battery_3.svg",
    "battery_4": "assets/icons/battery/battery_4.svg",
    "battery_5": "assets/icons/battery/battery_5.svg",
    "charging": "assets/icons/battery/charging.svg"
  }
}
```

## 4. 编译期配置（`builder` 模块联动）

```rust
{
  output_dir: PathBuf::from("src/assets"),  // 编译输出目录（固定）
  sentences_dir: PathBuf::from("../sentences-bundle/sentences"),  // 格言文件目录
  categories_path: PathBuf::from("../sentences-bundle/categories.json"),  // 格言分类配置
  font_path: PathBuf::from("assets/fonts/MapleMono-NF-CN-Regular.ttf"),  // 字体文件路径
  font_size_configs: vec![  // 支持的字体大小（仅以下三种）
    FontSizeConfig::new("Small", 16),  // 小号：16px
    FontSizeConfig::new("Medium", 24), // 中号：24px
    FontSizeConfig::new("Large", 40),  // 大号：40px
  ],
  local_icon_categories: vec![  // 本地图标分类配置
    LocalIconCategoryConfig {
      category: "battery".to_string(),
      dir: PathBuf::from("assets/icons/battery"),
      enum_name: "BatteryIcon".to_string(),
      width: 32,  // 固定宽度（不可修改）
      height: 32, // 固定高度（不可修改）
    },
    LocalIconCategoryConfig {
      category: "network".to_string(),
      dir: PathBuf::from("assets/icons/network"),
      enum_name: "NetworkIcon".to_string(),
      width: 32,
      height: 32,
    },
    LocalIconCategoryConfig {
      category: "time_digit".to_string(),
      dir: PathBuf::from("assets/icons/time_digit"),
      enum_name: "TimeDigitIcon".to_string(),
      width: 48,
      height: 64,
    },
  ],
  weather_icon_config: WeatherIconConfig {  // 天气图标配置
    dir: PathBuf::from("../Icons"),
    list_path: PathBuf::from("../Icons/icons-list.json"),
    enum_name: "WeatherIcon".to_string(),
    width: 64,  // 固定宽度
    height: 64, // 固定高度
  },
  main_layout_path: PathBuf::from("assets/layout/main.yaml"),  // 主布局文件路径（固定）
}
```

## 5. 布局规则核心总结

### 5.1 节点继承规则

#### 5.1.1 几何属性继承（`rect`/`weight`）

- 未指定 `rect`（位置+尺寸）：完全继承父容器的 `rect` 属性
- 部分指定 `rect`（如仅指定 `x` 和 `width`）：未指定字段（`y`/`height`）继承父容器对应值
- 存在 `weight` 属性：优先按权重分配父容器空间，忽略 `rect` 中的 `width`/`height`

#### 5.1.2 样式属性继承（如 `font_size`）

- 父节点指定样式属性，子节点未指定时自动继承
- 子节点指定样式属性时，覆盖父节点对应属性（就近原则）

```yaml
root_container:
  font_size: Medium  # 全局默认
  child_container:
    font_size: Large  # 覆盖父级
    text_node: {}     # 继承 Large 字体
```

#### 5.1.3 对齐方式默认值（未指定时）

- 容器节点：`alignment: start`（水平左对齐），`vertical_alignment: start`（垂直上对齐）
- 文本节点：`alignment: start`，`vertical_alignment: center`（垂直居中）
- 图标节点：`alignment: center`（水平居中），`vertical_alignment: center`（垂直居中）

### 5.2 条件渲染规则

#### 5.2.1 支持的条件类型

```yaml
# 1. 存在性条件：判断字段是否存在/非空
condition: "{system.config.wifi.ssid}"    # 字段存在且非空
condition: "{system.config.wifi.ssid?}"   # 字段存在（可为空）

# 2. 布尔条件：直接使用布尔字段
condition: "{system.is_charging}"         # 字段为true
condition: "!{system.is_charging}"        # 字段为false

# 3. 比较条件：数值/字符串/长度比较
condition: "{system.battery_level} < 20"
condition: '{weather.daily_weather[0].weather_desc} == "晴"'
condition: "{hitokoto.content.length} > 10"

# 4. 组合条件：使用逻辑运算符拼接
condition: "({system.battery_level} > 20) && ({system.network_status} == 'connected')"

# 5. 正则匹配（提案，仅支持基础语法）
condition: '{hitokoto.from} matches "^[甲乙丙丁]$"'  # 匹配开头字符
```

#### 5.2.2 条件求值规则

1. 求值顺序：同一容器内的子节点，按配置顺序从上到下求值
2. 互斥规则：同一容器内，第一个满足条件的节点显示，其余节点忽略（不渲染）
3. 嵌套规则：父节点条件不满足时，子节点不参与求值（直接隐藏）
4. 错误处理：表达式语法错误、字段不存在时，视为条件不满足（节点隐藏）

## 6. 功能要求

1. 代码补全：输入字段名（如 `content: "{d"`）时，联想全局数据源（如 `datetime` 及其子字段）
2. 语法校验：实时检查表达式语法（如缺失括号、运算符错误），并提示修正方案
3. 长度校验：输入 `id`/`content` 等字段时，实时显示当前长度，超出限制时标红警告
4. 图标联想：输入 `content: "{` 时，联想图标模块名（如 `weather_icon`），选择后联想对应图标键
5. 数据结构提示：hover 变量名（如 `{sensor.temperature}`）时，显示字段类型、范围及说明
6. 编译期配置校验：修改编译配置时，检查路径有效性、字体大小是否在支持列表内
7. 错误修复建议：检测到无效表达式（如类型不匹配、数组越界）时，提供具体修复方案