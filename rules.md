# epd-calendar项目布局规则
## 1. 基础约束
### 1.1 字符串长度限制
| 字段类型   | 最大长度 | 约束说明                                                                          |
|------------|----------|-----------------------------------------------------------------------------------|
| `id`       | 32字符   | 节点唯一标识符，仅允许小写字母（a-z）、数字（0-9）、下划线（_），首字符不能为数字 |
| `content`  | 128字符  | 仅支持文本内容、变量引用，超出部分按UTF-8完整字符截断                             |
| `condition`| 128字符  | 条件表达式（仅支持比较/逻辑/存在性检查），超出长度视为无效表达式                  |
| `icon_id`  | 64字符   | 专用于图标渲染，格式为 `{图标模块}:{图标键}`，需与 `icon_generator.rs` 联动       |

### 1.2 表达式约束
- 变量引用用`{}`包裹，嵌套层级≤2层；
- 花括号内禁止`+、-、*、/、%、?`运算符，外部允许（无需转义，作为字符串）；
- 支持逻辑运算符`&&、||、!`，比较运算符`>、<、==、!=`，存在性检查`?`（仅字段后）。

## 2. 节点规则
### 2.1 节点类型 & 允许属性
| 节点类型 | 必选属性                  | 可选属性                                                                 | 禁止属性                     |
|----------|---------------------------|--------------------------------------------------------------------------|------------------------------|
| container | id、type、position、anchor | direction、alignment、vertical_alignment、children、weight、condition、width、height | start、end、thickness、is_absolute、icon_id、content、font_size、max_width、max_lines |
| text     | id、type、position、anchor、content | font_size、alignment、vertical_alignment、max_width、max_lines、weight、is_absolute、condition、width、height | start、end、thickness、icon_id |
| icon     | id、type、position、anchor、icon_id | alignment、vertical_alignment、weight、is_absolute、condition、width、height | start、end、thickness、content、font_size、max_width、max_lines |
| line     | id、type、start、end、thickness | is_absolute、condition                                                  | position、anchor、width、height、icon_id、content、font_size、max_width、max_lines、weight、direction、alignment、vertical_alignment |

### 2.2 属性取值约束
| 属性          | 取值范围/规则                                                                 |
|---------------|------------------------------------------------------------------------------|
| direction     | 仅`horizontal`/`vertical`（container节点）                                   |
| alignment     | 仅`center`/`left`/`right`                                                    |
| vertical_alignment | 仅`center`/`top`/`bottom`                                                |
| anchor        | 仅`top-left`/`top-center`/`top-right`/`center-left`/`center`/`center-right`/`bottom-left`/`bottom-center`/`bottom-right`，默认`top-left` |
| font_size     | 支持数字（px）或字符串（如`Small`/`Medium`/`Large`/`16px`/`24px`），内置映射：Small=16、Medium=24、Large=40，支持扩展 |
| max_width     | 0≤值≤800（整数，text节点）                                                  |
| max_lines     | 1≤值≤5（整数，text节点）                                                    |
| weight        | 0<值≤10（数字，仅container子节点可用，父容器需指定direction）                |
| thickness     | 1≤值≤3（整数，line节点）                                                    |
| start/end     | 二元数组`[x,y]`，x∈[0,800]、y∈[0,480]（line节点）                            |
| position      | 二元数组`[x,y]`，x/y≥0；绝对布局时x∈[0,800]、y∈[0,480]                      |
| width         | ≥0（数字）；text/icon节点可选，默认按自带大小规则计算                        |
| height        | ≥0（数字）；text/icon节点可选，默认按自带大小规则计算                        |
| is_absolute   | 布尔值`true/false`，仅text/icon/line节点可用                                |

### 2.3 自带大小规则（text/icon节点）
#### 2.3.1 Text节点
- 高度默认值：与font_size强绑定，`font_size`数值即为默认高度（如Small=16→高度16，24px→高度24）；
- 宽度默认值：按`content`字符数 × (font_size×0.6) 计算（单字符宽度≈字体大小的60%），不超过max_width（若配置）；
- 若指定width/height，覆盖默认值，超出部分自动裁剪文本。

#### 2.3.2 Icon节点
- 宽度/高度默认值：使用图标原生尺寸（由icon_id对应的图标定义，如digit_icon:0→24x24，weather_icon:sun→32x32）；
- 若指定width/height，按指定尺寸缩放图标，保持宽高比。

### 2.4 容器大小计算规则
- 若container未指定width/height，默认按子节点的位置+大小自动计算：
  1. 水平布局（horizontal）：宽度=最右侧子节点的position.x + 子节点width；高度=所有子节点height的最大值；
  2. 垂直布局（vertical）：高度=最底部子节点的position.y + 子节点height；宽度=所有子节点width的最大值；
  3. 绝对布局子节点不参与父容器大小计算；
- 若指定width/height，子节点超出部分自动裁剪。

### 2.5 布局约束
1. 绝对布局（is_absolute=true）：text/icon的position为屏幕绝对坐标，需在800x480范围内；line的start/end为屏幕绝对坐标；
2. 相对布局（is_absolute=false）：text/icon的position为父容器内相对坐标，非负即可；
3. 锚点规则：position为元素锚点的坐标（如anchor=center时，position[x,y]是元素中心坐标；anchor=bottom-right时是元素右下角坐标）；
4. line节点参考CSS `line` 规范，通过start/end定义线段两端点，thickness定义线宽。

## 3. 全局禁止规则
- 花括号内禁止算术运算（+、-、*、/、%）和三元运算符（? :）；
- 节点嵌套层级≤10层，container的children数量≤20；
- 所有属性值禁止空字符串/空数组（必填属性）。