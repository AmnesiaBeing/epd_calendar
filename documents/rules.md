# 适用于嵌入式系统的布局系统规则
本文档描述了一个适用于嵌入式系统的简单布局系统。该系统旨在编译期完成尽可能多的布局计算，以减少运行时开销。布局系统支持两种布局模式：流式布局和绝对布局。

# BuildConfig 构建配置
BuildConfig用于在配置阶段描述所有资源目录的位置。

## BuildConfig 结构
| 字段名                | 类型                                 | 默认值                                               | 描述                                             |
| --------------------- | ------------------------------------ | ---------------------------------------------------- | ------------------------------------------------ |
| output_dir            | PathBuf                              | src/assets                                           | 构建输出目录，所有生成的资源文件都会放到此目录   |
| sentences_dir         | PathBuf                              | ../sentences-bundle/sentences                        | 格言文件目录，包含日历中使用的各种格言           |
| categories_path       | PathBuf                              | ../sentences-bundle/categories.json                  | 分类配置文件路径，定义句子的分类信息             |
| font_path             | PathBuf                              | assets/fonts/MapleMono-NF-CN-Regular.ttf             | 字体文件路径，用于生成字体位图                   |
| font_size_configs     | Vec\<FontSizeConfig\>                | 包含3个配置项<br/>(Small:16px, Medium:24px, Large:40px) | 字体尺寸配置列表，定义要生成的不同字体大小       |
| icon_categories       | Vec\<IconCategoryConfig\>            | 详细配置见下文                                       | 图标分类配置列表，定义不同类别的图标资源         |
| weather_icon_config   | WeatherIconConfig                    | 详细配置见下文                                       | 天气图标配置，定义天气图标的生成规则             |
| main_layout_path      | PathBuf                              | assets/layout/main.yaml                              | 主布局配置文件路径，定义界面布局结构             |

> 注意：所有相对路径都是相对于当前项目目录（实际解析时基于工程根目录）。

## FontSizeConfig 字体尺寸配置
| 字段名 | 类型   | 默认值         | 描述                                                                 |
| ------ | ------ | -------------- | -------------------------------------------------------------------- |
| name   | String | 根据配置项不同 | 字体尺寸的名称标识，用于生成的代码中；命名规范：SnakeCase（如xsmall/small/medium/large/xlarge） |
| size   | u16    | 根据配置项不同 | 字体的像素高度；编译期校验：值≥1，否则抛出错误                       |

> 编译期生成Rust枚举时，name自动转换为PascalCase（如XSmall/Small/Medium/Large/XLarge），仅允许字母、数字、下划线，首字符不能为数字。

## IconCategoryConfig 图标分类配置
| 字段名      | 类型    | 默认值       | 描述                                                                 |
| ----------- | ------- | ------------ | -------------------------------------------------------------------- |
| category    | String  | 根据分类不同 | 图标的分类名称，用于匹配icon_id前缀；命名规范：SnakeCase（如battery/time_digit） |
| dir         | PathBuf | 根据分类不同 | 图标文件所在的目录路径                                               |
| enum_name   | String  | 根据分类不同 | 生成的Rust枚举类型名称；命名规范：PascalCase（如BatteryIcon/TimeDigitIcon） |
| width       | u16     | 根据分类不同 | 图标的固定宽度（像素）；编译期校验：值≥1，否则抛出错误               |
| height      | u16     | 根据分类不同 | 图标的固定高度（像素）；编译期校验：值≥1，否则抛出错误               |

### 默认分类配置
| 分类        | 目录                        | 枚举名          | 宽度 | 高度 |
| ----------- | --------------------------- | --------------- | ---- | ---- |
| battery     | assets/icons/battery        | BatteryIcon     | 32   | 32   |
| network     | assets/icons/network        | NetworkIcon     | 32   | 32   |
| time_digit  | assets/icons/time_digit     | TimeDigitIcon   | 48   | 64   |

## WeatherIconConfig 天气图标配置
| 字段名      | 类型    | 默认值                   | 描述                                                                 |
| ----------- | ------- | ------------------------ | -------------------------------------------------------------------- |
| dir         | PathBuf | ../Icons                 | 天气图标文件的根目录                                                 |
| list_path   | PathBuf | ../Icons/icons-list.json | 图标清单JSON文件路径，包含天气图标的元数据                           |
| enum_name   | String  | WeatherIcon              | 生成的Rust枚举类型名称；命名规范：PascalCase，编译期校验符合Rust标识符规则 |
| width       | u16     | 64                       | 天气图标的固定宽度（像素）；编译期校验：值≥1，否则抛出错误           |
| height      | u16     | 64                       | 天气图标的固定高度（像素）；编译期校验：值≥1，否则抛出错误           |

> 天气图标归属统一图标规则，模块名固定为`weather`。

# 布局基础约束
## YAML字符串长度限制
| 字段类型  | 最大长度 | 约束说明                                                                                                                                                                                                   |
| --------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| id        | 32字符   | 编译期的元素唯一标识符，仅允许小写字母（a-z）、数字（0-9）、下划线（_），首字符不能为数字。在Release模式下会转换为NodeId类型（自增分配，全局唯一id数量≤65535），避免存储过多的字符串。                     |
| content   | 128字符  | 仅支持文本内容、变量引用，超出部分按UTF-8完整字符截断；编译期直接抛出错误，而非警告。                                                                                                                      |
| condition | 128字符  | 条件表达式（仅支持比较/逻辑/存在性检查），超出长度视为无效表达式，编译期抛出错误。                                                                                                                         |
| icon_id   | 64字符   | 专用于图标渲染，格式为{图标模块}:{图标键}，模块名固定（如time_digit/weather），键名可动态变更（如digit_{datetime.hour_tens}），需与BuildConfig联动；<br>编译期校验：必须包含且仅包含1个冒号，缺少/多余冒号、前缀未匹配均抛出错误。 |

## 表达式约束
### 通用规则
- 变量引用用 {} 包裹，嵌套层级 ≤ 2 层；
- 花括号内禁止 +、-、*、/、%、? 运算符，外部允许（无需转义，作为字符串）；
- 支持逻辑运算符 &&、||、!，比较运算符 >、<、==、!=，存在性检查 ?（仅字段后）。

### 表达式变量类型
整个表达式是一个字符串，其中使用花括号{}包裹的字符串在运行时可以转换为变量类型。
```Rust
pub type HeaplessString<const N: usize> = heapless::String<N>;
pub type HeaplessVec<T, const N: usize> = heapless::Vec<T, N>;

// 常量定义：与字段长度限制对齐
pub const KEY_LENGTH: usize = 32;   // 对应id字段最大长度
pub const VALUE_LENGTH: usize = 128;// 对应content字段最大长度

pub type CacheKey = HeaplessString<KEY_LENGTH>;
pub type CacheStringValue = HeaplessString<VALUE_LENGTH>;
pub type CacheKeyValueMap = BTreeMap<CacheKey, DynamicValue>;

pub enum DynamicValue {
    Boolean(bool), // 布尔值
    Integer(i32),  // 整数
    Float(f32),    // 浮点数
    String(CacheStringValue), // 字符串
}
```
> 注意：变量引用不存在时，默认为空字符串。

### 合法/非法示例
| 类型   | 示例                                  | 说明                     |
| ------ | ------------------------------------- | ------------------------ |
| 合法   | {user.age} > 18 && {device.online?}   | 存在性检查+比较+逻辑运算 |
| 合法   | {{weather.temp}} < 0                  | 2层嵌套变量引用          |
| 非法   | {{{a}.b}.c}                           | 嵌套层级超过2层          |
| 非法   | {a + b} == 10                         | 花括号内包含运算符       |

## 通用数值约束
| 数值类型     | 取值范围               | 说明                                                                      |
| ------------ | ---------------------- | ------------------------------------------------------------------------- |
| 屏幕绝对坐标 | x∈[0,800]、y∈[0,480]   | 绝对布局下的坐标上限，绝对坐标使用u16类型存储，无负值                     |
| 运算坐标     | 无限制                 | 运算坐标统一使用i16类型进行计算，允许负值；编译期转换为绝对坐标时，负值截断为0，超出上限截断为上限值 |
| 嵌套层级     | ≤10层                  | 所有元素的最大嵌套深度                                                    |
| 容器子元素   | ≤10个                  | Container元素的children数量上限                                           |

# YAML解析规则（编译期）
## 元素与字段类型
| 节点类型  | 必选属性              | 可选属性                                                                                                                                           |
| --------- | --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| container | id、type              | position、anchor、direction、alignment、vertical_alignment、children、weight、condition、width、height、layout                                       |
| text      | id、type、content     | position、font_size、alignment、vertical_alignment、max_width、max_height、weight、layout、condition、width、height                       |
| icon      | id、type、icon_id     | position、anchor、alignment、vertical_alignment、weight、layout、condition、width、height                                                            |
| line      | id、type              | thickness、layout（默认absolute）、condition、start、end                                                                                           |
| rectangle | id、type              | position、anchor、width、height、_thickness、layout、condition                                           |

### 注意事项
1. Text元素没有anchor属性，position为文本边界框的参考坐标，最终位置需结合对齐属性计算；
2. Text元素只支持从左到右，从上到下布局；
3. Circle元素已删除，不再支持；
4. 定位超出容器时，会被屏幕裁剪（不会被容器裁剪）。

## 字段默认值规则
| 属性                              | 数值范围                                                                               | 适用元素类型                     | 补充说明                                                                                                                                                                                                                                                                                                   |
| --------------------------------- | -------------------------------------------------------------------------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| layout                            | 编译期&运行时：Option\<Layout Enum\><br />默认值：Flow                               | 除Line外所有元素                 | - layout: flow (默认) -> 受容器direction, alignment控制，可使用weight<br />- layout: absolute -> 使用position和anchor精确定位，忽略容器流式规则                                                                                                                                                        |
| position                          | 编译期&运行时：Option\<(i16, i16)\><br />允许为负值，表示超出容器边框<br />默认值：(0,0) | Container, Text, Icon, Rectangle | 当layout: flow时，此字段通常被忽略<br />当layout: absolute时，此为元素在父容器内的相对定位坐标<br />编译期尽可能完成相对位置计算（转换为u16绝对坐标），减少运行时开销                                                                                                                                       |
| anchor                            | 编译期&运行时：Option\<Anchor Enum\><br />默认值：Some(TopLeft)                     | 除Line、Text外所有元素           | 锚点为元素定位基准点（如Center表示position是元素中心坐标）                                                                                                                                                                                                                                                  |
| direction                         | 编译期：Option\<Direction Enum\><br />运行时：Direction Enum<br />默认值：Horizontal     | Container                        | 子元素布局方向：Horizontal（水平）、Vertical（垂直）                                                                                                                                                                                                                                                       |
| alignment/vertical_alignment      | 编译期：Option\<Alignment Enum\><br />运行时：Alignment Enum<br />默认值：Start       | 除Line外所有元素                 | 水平/垂直对齐方式：Start（左/顶）、Center（居中）、End（右/底）；输入允许left/top/right/bottom等描述，编译期统一转换为Start/Center/End                                                                                                                                                                      |
| weight                            | 编译期&运行时：Option\<f32\><br />默认值：0.0                                        | Container下除Line外的子元素      | 比例布局权重，仅当子元素的layout: flow且weight > 0.0时，才参与权重分配，Line元素不参与权重分配。<br />weight为0时：Text按内容/字体计算宽高，Icon补充默认尺寸，无编译期警告；<br />weight > 0的元素若总和为0，则所有子元素按固定尺寸布局（子元素自身尺寸之和）；<br />weight > 0需保证总和>0，否则按固定尺寸布局。 |
| width/height                      | 按「尺寸计算规则」计算                                                                 | 除Line外所有元素                 | 按「尺寸计算规则」计算；编译期校验：若显式设置则值≥1，否则修正为1并输出警告                                                                                                                                                                                                                                |
| font_size                         | 编译期：Option\<&str\><br />运行时：FontSize Enum                                    | Text                             | 只允许使用BuildConfig下设定的字体大小，若字体不存在，按第一种字体类型转换；编译期校验名称合法性。                                                                                                                                                                                                          |
| max_width/max_height              | 编译期：Option\<u16\><br />无默认值                                                  | 除Line外所有元素                 | 仅当配置该字段时参与尺寸计算；未配置时，文本/图标按内容/默认尺寸计算，容器按子元素尺寸计算。                                                                                                                                                                                                               |
| thickness                         | 编译期：Option\<u16\><br />默认值：1                                                 | Line, Rectangle                  | 线条/描边宽度；编译期校验：值≥1，否则修正为1并输出警告。                                                                                                                                                                                                                                                  |

### 可省略字段（编译期+运行时自动补全）
所有默认值字段可省略，编译期自动补全，示例如下：
```YAML
# 完整属性
- node:
    type: icon
    id: hour_tens
    position: [0, 0]
    anchor: TopLeft
    width: 48
    height: 64
    icon_id: "time_digit:digit_{datetime.hour_tens}"
  weight: 1.0

# 简化后属性（编译期自动补全默认值）
- node:
    type: icon
    id: hour_tens
    icon_id: "time_digit:digit_{datetime.hour_tens}"
```

## 图标尺寸自动补充规则
编译期解析icon_id时，自动补充默认宽高：
1. 解析icon_id前缀（如time_digit/weather）匹配IconCategoryConfig/WeatherIconConfig的模块名；
2. 匹配成功则使用对应width/height作为默认值；
3. 未匹配/格式错误（如缺少冒号、前缀不存在）编译期直接抛出异常；

### 示例
| icon_id                | 匹配规则               | 结果         |
| ---------------------- | ---------------------- | ------------ |
| time_digit:digit_0     | 匹配IconCategoryConfig | 宽48、高64   |
| weather:sun            | 匹配WeatherIconConfig  | 宽64、高64   |
| battery:level_5        | 匹配IconCategoryConfig | 宽32、高32   |
| unknown:test           | 无匹配                 | 编译报错     |
| unknown_test           | 格式错误（无冒号）| 编译报错     |

# 编译期转换流程（YAML → 布局池）
## 解析阶段
1. 加载 YAML 文件，补全所有默认值字段；
2. 校验字段合法性（长度、格式、取值范围），非法字段直接抛出编译错误；
3. 解析 icon_id 并补充图标默认宽高：静态图标（无变量）补充为BuildConfig中指定的尺寸，动态图标（含变量）保留字符串并标记；
4. 解析后的公共类型示意（嵌入式内存对齐优化）：
```Rust
// 核心类型定义（嵌入式精简，内存对齐）
pub type NodeId = u16; // 自增分配，全局唯一，最大值65535

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Flow,
    Absolute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Start,
    Center,
    End,
}

/// 扁平化布局节点（无嵌套，所有子节点用NodeId引用）
#[derive(Debug, Clone)]
pub enum LayoutNode {
    Container(Container),
    Text(Text),
    Icon(Icon),
    Line(Line),
    Rectangle(Rectangle),
}

/// 其他元素定义请参见"元素与字段类型"
```

## 计算阶段
1. 锚点坐标转换（编译期完成，运行时无需计算）：
   设元素宽W、高H，anchor对应的position坐标为(Px, Py)（i16运算坐标），转换为u16绝对坐标（负值→0，超出上限→上限值）后：
   - TopLeft：(Px, Py)
   - Center：(Px - W/2, Py - H/2)
   - BottomRight：(Px - W, Py - H)
   > 仅绝对布局且无动态内容的元素，编译期完成锚点计算；动态内容元素（如含变量的icon_id）运行时计算。
2. 按「尺寸计算规则」计算Text/Icon/Container节点的宽高；
3. 尽可能将流式布局转换为绝对布局，减少运行时计算量；

## 池化阶段
1. 将所有节点扁平化存储到LayoutPool：
   - 编译期临时存储：`alloc::vec::Vec<(NodeId, NodeId, LayoutNode)>`（node_id: 节点ID，parent_node_id: 父节点ID，layout_node: 节点数据）；
   - 最终转换为静态数组，无堆分配；
2. 保留父容器ID关联，便于运行时快速查找约束；
3. 生成运行时只读的静态布局数组；
4. 编译期输出警告（如布局异常、数值修正），错误直接终止编译；

## 生成阶段
1. 生成 `generated_layouts.rs` 文件；
2. 所有字符串常量转为 `&'static str`，数组转为 `&'static [T]`（嵌入式友好，无堆分配）；
3. 核心生成结构示例：
```Rust
// generated_layouts.rs
// 全局布局节点数组（包含节点ID、父节点ID、节点数据）
pub static LAYOUT_NODES: &'static [(NodeId, NodeId, LayoutNode)] = &[
    // 编译期自动生成示例
    // (0, u16::MAX, LayoutNode::Container(Container { id: 0, direction: Direction::Horizontal, .. })),
    // (1, 0, LayoutNode::Icon(Icon { id: 1, icon_id: "time_digit:digit_{datetime.hour_tens}", .. })),
];

// 空父节点标识（根节点）
pub const ROOT_PARENT_ID: NodeId = u16::MAX;
```

# 布局计算规则
## 尺寸计算规则
### 优先级
1. 显式设置：元素设置了width/height，优先使用设置值；
2. 自动计算：未显式设置时，按元素类型自动计算。

### 元素自动计算规则
| 元素类型 | 宽度计算规则                                                                 | 高度计算规则                                                                 |
| -------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Text     | 无设置时，根据内容、字体大小、max_width（若配置）计算（超出换行）| 无设置时，根据行数、字体大小、max_height（若配置）计算                       |
| Icon     | 无设置时，使用icon_id匹配的默认宽度                                           | 无设置时，使用icon_id匹配的默认高度                                           |
| Container| 水平方向：子元素宽度总和；垂直方向：子元素最大宽度                             | 垂直方向：子元素高度总和；水平方向：子元素最大高度                             |

### 权重分配规则
仅流式布局容器中weight>0的子元素参与剩余空间分配：
1. 剩余空间 = 容器总空间 - 非权重子元素总尺寸；（子元素间距默认0，无额外扣除）
2. 子元素分配尺寸 = 剩余空间 × (子元素weight / 总weight)；
3. 嵌入式适配：f32计算后四舍五入取整，编译期校验总分配尺寸≤剩余空间，超出则按比例缩减。

## 位置计算规则
### 绝对布局
根据position（i16）和anchor计算元素在容器中的左上角坐标，编译期转换为u16绝对坐标（负值→0，超出上限→上限值），尽可能在编译期完成计算。

### 流式布局
1. 容器根据direction排列子元素，子元素间距默认0；
2. 子元素根据alignment/vertical_alignment在分配的空间内对齐；
3. 子元素若为layout: absolute，忽略容器流式规则，使用绝对定位。

## 条件渲染规则
1. 每个元素最多配置一个condition表达式；
2. 表达式运行时求值，true则渲染，false则不渲染；
3. 不渲染的元素不占用布局空间（流式布局中）；
4. 编译期按“条件为true”计算布局，预留最大可能空间；

## 文本元素（Text）的特殊处理
文本元素通过基线（baseline）方式绘制，具体规则：

### 基线计算规则
1. 预存储字体元数据：font_size、字符位图宽高、bearing_x、bearing_y、advance_x；
2. 文本边界框高度 = 字体ascent + 字体descent（包含上行/下行）；
3. 基线位置 = 文本边界框顶部 + 字体ascent。

### 换行与截断规则
1. 若配置max_width，文本宽度不超过该值，超出部分换行；
2. 换行优先在标点后（中文：。，；！？；英文：.,;!?），无标点则按UTF-8字符截断；
3. 若配置max_height，文本高度不超过该值，超出部分不渲染。

### 垂直对齐规则
1. 先计算文本边界框尺寸；
2. 垂直对齐基于边界框：
   - Start：边界框顶部与容器顶部对齐；
   - Center：边界框中心与容器中心对齐；
   - End：边界框底部与容器底部对齐；
3. 绘制时基于基线对齐，保证文本显示一致性。