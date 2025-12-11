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
| 节点类型 | 必选属性                | 可选属性                                                                 | 禁止属性                     |
|----------|-------------------------|--------------------------------------------------------------------------|------------------------------|
| container | id、type、rect          | direction、alignment、vertical_alignment、children、weight、condition    | start、end、thickness、is_absolute、icon_id、content、font_size、max_width、max_lines |
| text     | id、type、rect、content | font_size、alignment、vertical_alignment、max_width、max_lines、weight、is_absolute、condition | start、end、thickness、icon_id |
| icon     | id、type、rect、icon_id | alignment、vertical_alignment、weight、is_absolute、condition            | start、end、thickness、content、font_size、max_width、max_lines |
| line     | id、type、start、end、thickness | is_absolute、condition                                                  | rect、icon_id、content、font_size、max_width、max_lines、weight、direction、alignment、vertical_alignment |

### 2.2 属性取值约束
| 属性          | 取值范围/规则                                                                 |
|---------------|------------------------------------------------------------------------------|
| direction     | 仅`horizontal`/`vertical`（container节点）                                   |
| alignment     | 仅`center`/`left`/`right`                                                    |
| vertical_alignment | 仅`center`/`top`/`bottom`                                                |
| font_size     | 仅`Small`(16px)/`Medium`(24px)/`Large`(40px)（text节点）                     |
| max_width     | 0≤值≤800（整数，text节点）                                                  |
| max_lines     | 1≤值≤5（整数，text节点）                                                    |
| weight        | 0<值≤10（数字，仅container子节点可用，父容器需指定direction）                |
| thickness     | 1≤值≤3（整数，line节点）                                                    |
| start/end     | 二元数组`[x,y]`，x∈[0,800]、y∈[0,480]（line节点）                            |
| rect          | 四元数组`[x,y,width,height]`，x/y/width/height≥0，绝对布局时x+width≤800、y+height≤480 |
| is_absolute   | 布尔值`true/false`，仅text/icon/line节点可用                                |

### 2.3 布局约束
1. 绝对布局（is_absolute=true）：text/icon/line的坐标（rect/start/end）为屏幕绝对坐标，需在800x480范围内；
2. 相对布局（is_absolute=false）：坐标为父容器内相对坐标，非负即可；
3. 居中布局：container通过`alignment:center + vertical_alignment:center`控制子节点居中；
4. line节点参考CSS `line` 规范，通过start/end定义线段两端点，thickness定义线宽。

## 3. 全局禁止规则
- 花括号内禁止算术运算（+、-、*、/、%）和三元运算符（? :）；
- 节点嵌套层级≤10层，container的children数量≤20；
- 所有属性值禁止空字符串/空数组（必填属性）。