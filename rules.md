# epd-calendar 项目布局规则

## 1. 基础约束

### 1.1 字符串长度限制

| 字段类型    | 最大长度 | 约束说明                                                                                               |
| ----------- | -------- | ------------------------------------------------------------------------------------------------------ |
| `id`        | 32 字符  | 节点唯一标识符，仅允许小写字母（a-z）、数字（0-9）、下划线（\_），首字符不能为数字                     |
| `content`   | 128 字符 | 仅支持文本内容、变量引用，超出部分按 UTF-8 完整字符截断                                                |
| `condition` | 128 字符 | 条件表达式（仅支持比较/逻辑/存在性检查），超出长度视为无效表达式                                       |
| `icon_id`   | 64 字符  | 专用于图标渲染，格式为 `{图标模块}:{图标键}`，需与 `icon_generator.rs` 联动（如 `time_digit:digit_0`） |

### 1.2 表达式约束

- 变量引用用 `{}` 包裹，嵌套层级 ≤ 2 层；
- 花括号内禁止 `+、-、*、/、%、?` 运算符，外部允许（无需转义，作为字符串）；
- 支持逻辑运算符 `&&、||、!`，比较运算符 `>、<、==、!=`，存在性检查 `?`（仅字段后）。

### 1.3 通用数值约束

| 数值类型   | 取值范围             | 说明                               |
| ---------- | -------------------- | ---------------------------------- |
| 屏幕坐标   | x∈[0,800]、y∈[0,480] | 绝对布局下的坐标上限               |
| 嵌套层级   | ≤10 层               | 所有节点的最大嵌套深度             |
| 容器子节点 | ≤20 个               | Container 节点的 children 数量上限 |

## 2. YAML 解析规则（编译期）

### 2.1 字段默认值规则

| 属性                 | 默认值                 | 适用节点类型                         | 补充说明                                                         |
| -------------------- | ---------------------- | ------------------------------------ | ---------------------------------------------------------------- |
| `position`           | `[0, 0]`               | container/text/icon/rectangle/circle | 相对布局下为父容器内坐标，绝对布局下为屏幕坐标                   |
| `anchor`             | `top-left`             | 除 line 外所有节点                   | 锚点为元素定位基准点（如 `center` 表示 position 是元素中心坐标） |
| `direction`          | `horizontal`           | container                            | 子节点布局方向                                                   |
| `alignment`          | `left`                 | container/text/icon                  | 水平对齐方式                                                     |
| `vertical_alignment` | `top`                  | container/text/icon                  | 垂直对齐方式                                                     |
| `weight`             | `1.0`                  | container 子节点                     | 比例布局权重，仅父容器指定 direction 时生效                      |
| `is_absolute`        | `false`                | text/icon/line                       | 是否启用绝对布局（忽略父容器布局规则）                           |
| `width`              | 按「自带大小规则」计算 | text/icon/container                  | 容器节点默认按子节点自动计算，text/icon 按内置规则计算           |
| `height`             | 按「自带大小规则」计算 | text/icon/container                  | 同 width                                                         |
| `font_size`          | `Medium`（对应 24px）  | text                                 | 内置映射：Small=16px、Medium=24px、Large=40px                    |
| `max_width`          | 800（屏幕宽度）        | text                                 | 文本自动换行的最大宽度                                           |
| `max_lines`          | 1                      | text                                 | 文本最大显示行数                                                 |
| `thickness`          | 1                      | line/rectangle/circle                | 线条/描边宽度                                                    |

### 2.2 YAML 简化写法规则

#### 2.2.1 可省略字段（编译期自动补全）

- 所有默认值字段可省略（如 `position: [0,0]`、`anchor: top-left`、`weight: 1.0`）；
- 示例：
  ```yaml
  # 简化前
  - node:
      type: icon
      id: hour_tens
      position: [0, 0]
      anchor: center
      width: 100
      height: 120
      icon_id: "time_digit:digit_{datetime.hour_tens}"
    weight: 1.0
  # 简化后（编译期自动补全默认值）
  - node:
      type: icon
      id: hour_tens
      icon_id: "time_digit:digit_{datetime.hour_tens}"
  ```

#### 2.2.2 图标尺寸自动补充规则

编译期解析 `icon_id` 时，通过 `BuildConfig` 自动补充默认宽高：

1. 解析 `icon_id` 前缀（如 `time_digit`）匹配 `local_icon_categories` 中的 `category`；
2. 匹配成功则使用对应 `width/height` 作为默认值；
3. 未匹配则使用天气图标默认尺寸（64x64）；
4. 示例：
   - `icon_id: "time_digit:digit_0"` → 默认宽 48、高 64（匹配 `time_digit` 分类）；
   - `icon_id: "weather:sun"` → 默认宽 64、高 64（天气图标配置）；
   - `icon_id: "battery:level_5"` → 默认宽 32、高 32（匹配 `battery` 分类）。

### 2.3 字段合法性校验规则

| 节点类型  | 必选属性             | 可选属性                                                                                                                        | 禁止属性                                                                                                                             |
| --------- | -------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| container | id、type             | position、anchor、direction、alignment、vertical_alignment、children、weight、condition、width、height                          | start、end、thickness、is_absolute、icon_id、content、font_size、max_width、max_lines                                                |
| text      | id、type、content    | position、anchor、font_size、alignment、vertical_alignment、max_width、max_lines、weight、is_absolute、condition、width、height | start、end、thickness、icon_id                                                                                                       |
| icon      | id、type、icon_id    | position、anchor、alignment、vertical_alignment、weight、is_absolute、condition、width、height                                  | start、end、thickness、content、font_size、max_width、max_lines                                                                      |
| line      | id、type、start、end | thickness、is_absolute、condition                                                                                               | position、anchor、width、height、icon_id、content、font_size、max_width、max_lines、weight、direction、alignment、vertical_alignment |
| rectangle | id、type             | position、anchor、width、height、fill_importance、stroke_importance、stroke_thickness、is_absolute、condition                   | start、end、icon_id、content、font_size、max_width、max_lines、weight、direction                                                     |
| circle    | id、type、radius     | position、anchor、fill_importance、stroke_importance、stroke_thickness、is_absolute、condition                                  | start、end、width、height、icon_id、content、font_size、max_width、max_lines、weight、direction                                      |

## 3. 布局计算规则（编译期+运行时）

### 3.1 自带大小规则（Text/Icon 节点）

#### 3.1.1 Text 节点（默认宽高计算）

1. **高度默认值**：与 `font_size` 强绑定，`font_size` 数值即为高度（如 Small=16 → 高度 16，24px → 高度 24）；
2. **宽度默认值**：`字符数 × (font_size × 0.6)`（单字符宽度 ≈ 字体大小的 60%），不超过 `max_width`；
3. 若指定 `width/height`，覆盖默认值，超出部分自动裁剪文本。

#### 3.1.2 Icon 节点（默认宽高计算）

1. **默认值来源**：
   - 优先匹配 `BuildConfig` 中 `local_icon_categories`/`weather_icon_config` 的 `width/height`；
   - 示例：`icon_id: "time_digit:digit_0"` → 宽 48、高 64（time_digit 分类配置）；
2. 若指定 `width/height`，按指定尺寸缩放图标（保持宽高比）；
3. 未匹配到配置时，默认使用 32x32（兜底值）。

### 3.2 Container 节点大小计算规则

#### 3.2.1 自动计算（未指定 width/height）

1. **水平布局（horizontal）**：
   - 宽度 = 最右侧子节点的 `position.x + 子节点width`；
   - 高度 = 所有子节点 `height` 的最大值；
2. **垂直布局（vertical）**：
   - 高度 = 最底部子节点的 `position.y + 子节点height`；
   - 宽度 = 所有子节点 `width` 的最大值；
3. 绝对布局子节点（`is_absolute=true`）不参与父容器大小计算；

#### 3.2.2 手动指定（已配置 width/height）

- 子节点超出容器范围的部分自动裁剪；
- 子节点相对坐标超过容器宽高时，视为布局异常（编译期给出警告）。

### 3.3 锚点与坐标映射规则

| 锚点类型      | position 对应元素位置 | 坐标计算示例（元素宽 w=100，高 h=50） |
| ------------- | --------------------- | ------------------------------------- |
| top-left      | 左上角                | 元素左上角 = (x, y)                   |
| top-center    | 上边缘中点            | 元素左上角 = (x - w/2, y)             |
| top-right     | 右上角                | 元素左上角 = (x - w, y)               |
| center-left   | 左边缘中点            | 元素左上角 = (x, y - h/2)             |
| center        | 中心                  | 元素左上角 = (x - w/2, y - h/2)       |
| center-right  | 右边缘中点            | 元素左上角 = (x - w, y - h/2)         |
| bottom-left   | 左下角                | 元素左上角 = (x, y - h)               |
| bottom-center | 下边缘中点            | 元素左上角 = (x - w/2, y - h)         |
| bottom-right  | 右下角                | 元素左上角 = (x - w, y - h)           |

### 3.4 布局模式规则

#### 3.4.1 相对布局（is_absolute=false）

- Text/Icon 节点：`position` 为父容器内相对坐标（非负即可）；
- Container 子节点：按 `direction` 自动排列，`weight` 决定占比（如 weight=2 的子节点占比是 weight=1 的 2 倍）；

#### 3.4.2 绝对布局（is_absolute=true）

- Text/Icon 节点：`position` 为屏幕绝对坐标（需满足 x∈[0,800]、y∈[0,480]）；
- Line 节点：`start/end` 为屏幕绝对坐标（需满足坐标范围约束）；
- 绝对布局节点不受父容器 `direction/alignment` 影响；

### 3.5 特殊节点计算规则

#### 3.5.1 Line 节点

- `start/end` 定义线段两端点，`thickness` 定义线宽（1-3px）；
- 绝对布局时 `start/end` 必须在屏幕范围内，相对布局时为父容器内坐标；

#### 3.5.2 Rectangle/Circle 节点

1. **Rectangle**：
   - 宽高为必填项（编译期校验），默认锚点 `top-left`；
   - 描边宽度 `stroke_thickness` 范围 1-3px，超出视为无效（强制设为 3）；
2. **Circle**：
   - 半径范围 1-400px（屏幕半宽），默认锚点 `center`；
   - 描边宽度规则同 Rectangle；

## 4. 全局禁止规则

1. 花括号内禁止算术运算（+、-、\*、/、%）和三元运算符（? :）；
2. 所有节点嵌套层级 ≤ 10 层，Container 节点的 children 数量 ≤ 20；
3. 必填属性禁止空字符串/空数组（如 text 节点的 `content` 不能为空）；
4. Icon 节点的 `icon_id` 必须匹配 `{模块}:{键}` 格式，否则编译期报错；
5. Font_size 仅支持数字（px）、`Small/Medium/Large` 或带 px 后缀的字符串（如 `16px`），其他值视为无效；
6. 禁止设置 `weight ≤ 0` 或 `weight > 10`，编译期自动修正为 0.0001 或 10；
7. Line 节点的 `thickness` 超出 1-3px 范围时，编译期自动修正为 1 或 3。

## 5. 编译期转换流程（YAML → 布局池）

1. **解析阶段**：
   - 加载 YAML 文件，补全所有默认值字段；
   - 校验字段合法性（长度、格式、取值范围）；
   - 解析 `icon_id` 并补充图标默认宽高；
2. **计算阶段**：
   - 按锚点规则计算元素实际左上角坐标；
   - 按自带大小规则计算 Text/Icon 节点默认宽高；
   - 按容器规则计算 Container 节点自动宽高；
3. **池化阶段**：
   - 将所有节点扁平化存储到 LayoutPool；
   - 生成运行时只读的静态布局数组；
   - 编译期时输出警告（如布局异常、数值修正）；
4. **生成阶段**：
   - 生成 `generated_layouts.rs` 文件；
   - 所有字符串常量转为 `&'static str`，数组转为 `&'static [T]`（嵌入式友好）。
