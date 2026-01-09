# 800x480嵌入式HTML解析与模拟显示

# 目标说明
- 根据下述规则，生成`parse_html.py`和`render.py`
- `parse_html.py`负责解析HTML文件，生成python渲染与计算代码文件
  - 默认参数是`--input input.html --output output.py`
  - 输入的HTML文件必须符合下述"HTML/CSS使用规则"
  - 输出的`output.py`包含一些计算时运算的常量
- `render.py`负责绘制部分，可以通过PIL库直接将布局渲染成PNG文件
  - `render.py`默认参数是`--input output.py --output output.png`
- （重要）如果对需求有疑问，请先询问，而不是生成代码

## HTML/CSS使用规则（强制约束）
### 基础结构规则
1. **根容器必选**：必须包含 `<div class="page-container">` 作为唯一根节点，尺寸固定为800x480，禁止嵌套多个根容器；
2. **元素层级限制**：最大嵌套层级10层（从`page-container`开始计数），层级结构需符合“根容器 → Flex容器 → 子项组 → 包装器 → 显示元素”的逻辑；
3. **动态显隐规则**：仅允许通过 `display: block/none` 控制元素显隐（仅img包装器支持动态占位符`{display_占位符}`），显隐元素会影响Flex布局的剩余空间计算；
4. **禁止固定定位**：禁止使用`position: fixed/absolute`（仅保留Flex布局的动态定位），所有元素位置由Flex规则动态计算。

### CSS规则（强制约束）
#### 全局样式必选内容
以下内容必须在HTML文件的`<head>`中全局定义，禁止在`<body>`中重复定义：
```css
/* 基础重置&字体引入 */
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}
@font-face {
    font-family: 'MapleMono';
    src: url('assets/fonts/MapleMono-NF-CN-Regular.ttf') format('truetype');
    font-weight: normal;
    font-style: normal;
}
/* 根容器 */
root_container {
    position: relative;
    width: 800px;
    height: 480px;
    background-color: #FFFFFF;
    border: 1px solid #000000;
    font-family: 'MapleMono';
    display: flex;
    flex-direction: column;
    padding: 10px;
}
```
- 字体规则：`@font-face` 必须存在，渲染期直接加载该字体；
- 字体文件路径：仅支持相对路径（相对于HTML文件），格式仅支持TTF（嵌入式通用格式）。
- 不要使用`cssutils`之类的过时的库来解析CSS，请考虑更现代的库，或者手动解析

#### Flex布局支持规则
| Flex属性          | 支持值                          | 动态计算规则                                                                 |
|-------------------|---------------------------------|------------------------------------------------------------------------------|
| flex-direction    | row/column                      | 运行时按该值确定布局方向，子元素沿行/列排列                                  |
| justify-content   | flex-start/center/space-around/space-between | 运行时根据子元素总尺寸与父容器尺寸的差值，动态计算子元素水平/垂直偏移         |
| align-items       | flex-start/center/flex-end      | 运行时根据子元素尺寸与父容器尺寸的差值，动态计算子元素垂直/水平偏移           |
| flex-basis        | px（仅数值）                    | 优先作为Flex容器/子元素的基础尺寸，覆盖width/height                          |
| flex-grow         | 0/1                             | 仅支持0（不占剩余空间）、1（占剩余空间），多个flex-grow:1的元素均分剩余空间   |
| display           | flex/block/none/{占位符}        | none的元素不参与Flex计算，{占位符}运行时替换为block/none后再计算             |

#### 显示元素样式规则
| 元素类型 | 必选样式                | 可选样式                          | 渲染规则                     |
|----------|-------------------------|-----------------------------------|------------------------------|
| 文本元素 | font-size(px)、line-height(px) | text-align(left/center/right)     | 按行高换行，单个字符级断行   |
| 空白容器 | width/height(px/%)、border-width(px) | border-style(solid)               | 绘制边框 + 填充背景色        |
| 图片元素 | width/height(px/%)      | border-width(px)、border-style(solid) | 仅画矩形框示意，标注class/尺寸 |

### 其他布局支持规则
对于position属性，只支持默认的static和absolute两种，其他的布局方式不应该被支持
考虑z-index属性，z-index越高，越晚绘制

### 动态占位符规则（仅支持以下场景）
| 占位符位置                | 格式要求               | 运行时处理逻辑                                                                 |
|---------------------------|------------------------|------------------------------------------------------------------------------|
| img包装器的display样式    | {dynamic_display_*}    | 替换为block/none，none时该元素不参与Flex布局计算                             |
| 文本元素的content          | {dynamic_text_*}       | 替换为实际文本，文本长度变化会影响换行，但不影响Flex容器的基础尺寸             |
| img元素的src属性           | {dynamic_img_src_*}    | 仅保留占位符，渲染时忽略src，仅在计算位置画框（模拟环境无需实际加载图片）     |

## 解析示例
### 布局示例1
```html
/* 这里补充全局样式必选内容 */

.box-1 {
    position: absolute;
    top: 100px;
    left: 100px;
    width: 100px;
    height: 100px;
    border-width: 1px;
    border-style: solid;
    padding: 10px; /* 虽然这里声明了padding和margin，但因为是绝对布局，不会对其他元素产生影响 */
    margin: 10px;
}

<body>
    <div class="page-container"> <!-- 根容器 -->
        <div class="box-1"/> <!-- 示例容器 -->
    </div>
</body>
```

这里应当生成PIL的python代码，在800*480的画布上绘制4条线，线条的宽度为1，分别是：
- 从(box-1.left, box-1.top)到(box-1.left+box-1.width, box-1.top)
- 从(box-1.left+box-1.width, box-1.top)到(box-1.left+box-1.width, box-1.top+box-1.height)
- 从(box-1.left+box-1.width, box-1.top+box-1.height)到(box-1.left, box-1.top+box-1.height)
- 从(box-1.left, box-1.top+box-1.height)到(box-1.left, box-1.top)

### 布局示例2（嵌套布局）
```html
/* 这里补充全局样式必选内容 */

.box-1 {
    top: 100px; /* 非绝对布局 top、left属性不应该生效 */
    left: 100px;
    width: 100px;
    height: 100px;
    border-width: 1px;
    border-style: solid;
    padding: 10px; /* 这里导致下一个容器距离这个容器有(10px,10px) */
    margin: 10px; /* 这里导致该容器距离根容器左上角(10px,10px) */
}

.box-2 {
    width: 100px;
    height: 100px;
    border-width: 1px;
    border-style: solid;
    padding: 10px;
    margin: 10px; /* 这个容器本身需要距离上一个容器有偏移(10px,10px) */
}

<body>
    <div class="page-container"> <!-- 根容器 -->
        <div class="box-1"> <!-- 示例容器1 -->
            <div class="box-2"/> <!-- 示例容器2 -->
        </div>
    </div>
</body>
```

这里应当生成PIL的python代码，在800*480的画布上绘制8条线，线条的宽度为1，分别是：
- 从(box-1.margin-left,box-1.margin-top)到(box-1.margin-left+box-1.width,box-1.margin-top)
- 从(box-1.margin-left+box-1.width,box-1.margin-top)到(box-1.margin-left+box-1.width,box-1.margin-top+box-1.height)
- 从(box-1.margin-left+box-1.width,box-1.margin-top+box-1.height)到(box-1.margin-left,box-1.margin-top+box-1.height)
- 从(box-1.margin-left,box-1.margin-top+box-1.height)到(box-1.margin-left,box-1.margin-top)
注：box-2的位置计算基于box-1的padding和margin，定义如下：
box-2.left = box-1.margin-left+box-1.padding-left+box-2.margin-left
box-2.top = box-1.margin-top+box-1.padding-top+box-2.margin-top
- 从(box-2.left,box-2.top)到(box-2.left+box-2.width,box-2.top)
- 从(box-2.left+box-2.width,box-2.top)到(box-2.left+box-2.width,box-2.top+box-2.height)
- 从(box-2.left+box-2.width,box-2.top+box-2.height)到(box-2.left,box-2.top+box-2.height)
- 从(box-2.left,box-2.top+box-2.height)到(box-2.left,box-2.top)

### 布局示例3
```html
/* 这里补充全局样式必选内容 */

.box-1 {
    width: 100px;
    height: 100px;
    border-width: 1px;
    border-style: solid;
    padding: 10px; /* 内部没有东西，这个属性是无效的 */
    margin: 10px; /* 这里导致该容器距离根容器左上角(10px,10px) */
}

.box-2 {
    width: 100px;
    /* height: 100px; */ /* 默认情况下，高度为0 */
    border-width: 1px;
    border-style: solid;
    padding: 10px; /* 这个padding值会影像实际高度 */
    margin: 10px; /* 这个容器本身需要距离上一个容器有偏移(10px,10px) */
}

<body>
    <div class="page-container">
        <div class="box-1"></div>
        <div class="box-2"></div>
    </div>
</body>
```

这里应当生成PIL的python代码，在800*480的画布上绘制8条线，线条的宽度为1，分别是：
- 从(box-1.margin-left,box-1.margin-top)到(box-1.margin-left+box-1.width,box-1.margin-top)
- 从(box-1.margin-left+box-1.width,box-1.margin-top)到(box-1.margin-left+box-1.width,box-1.margin-top+box-1.height)
- 从(box-1.margin-left+box-1.width,box-1.margin-top+box-1.height)到(box-1.margin-left,box-1.margin-top+box-1.height)
- 从(box-1.margin-left,box-1.margin-top+box-1.height)到(box-1.margin-left,box-1.margin-top)
注：考虑到根容器是一个垂直的flex布局，box-2的垂直位置计算基于box-1的margin，定义如下：
box-2.top = box-1.margin-top+box-1.height+box-1.margin-bottom+box-2.margin-top
box-2.left = box-2.margin-left
另外，box-2的高度在原属性中没有定义，但是原属性定义了padding，所以：
box-2.height = box-2.padding-top+box-2.padding-bottom
- 从(box-2.left,box-2.top)到(box-2.left+box-2.width,box-2.top)
- 从(box-2.left+box-2.width,box-2.top)到(box-2.left+box-2.width,box-2.top+box-2.height)
- 从(box-2.left+box-2.width,box-2.top+box-2.height)到(box-2.left,box-2.top+box-2.height)
- 从(box-2.left,box-2.top+box-2.height)到(box-2.left,box-2.top)

### 布局示例4
```html
/* 这里补充全局样式必选内容 */

.box-1 {
    width: 100px;
    height: 100px;
    border-width: 1px;
    border-style: solid;
    padding: 10px; /* 内部没有东西，这个属性是无效的 */
    margin: 10px; /* 这里导致该容器距离根容器左上角(10px,10px) */
}

.box-2 {
    width: 100%; /* 这里导致box-2的宽度为根容器的宽度 */
    /* height: 100px; */
    flex: 1; /* 这里导致box-2的高度为容器的高度减去box-1的高度和对应的margin */
    border-width: 1px;
    border-style: solid;
    padding: 10px; /* 内部没有东西，这个属性是无效的 */
    margin: 10px; /* 这个容器本身需要距离上一个容器有偏移(10px,10px) */
}

<body>
    <div class="page-container">
        <div class="box-1"></div>
        <div class="box-2"></div>
    </div>
</body>
```

这里应当生成PIL的python代码，在800*480的画布上绘制8条线，线条的宽度为1，分别是：
- 从(box-1.margin-left,box-1.margin-top)到(box-1.margin-left+box-1.width,box-1.margin-top)
- 从(box-1.margin-left+box-1.width,box-1.margin-top)到(box-1.margin-left+box-1.width,box-1.margin-top+box-1.height)
- 从(box-1.margin-left+box-1.width,box-1.margin-top+box-1.height)到(box-1.margin-left,box-1.margin-top+box-1.height)
- 从(box-1.margin-left,box-1.margin-top+box-1.height)到(box-1.margin-left,box-1.margin-top)
注：考虑到根容器是一个垂直的flex布局，box-2的垂直位置计算基于box-1的margin，定义如下：
box-2.top = box-1.margin-top+box-1.height+box-1.margin-bottom+box-2.margin-top
box-2.left = box-2.margin-left
另外，box-2的高度在原属性中没有定义，但是原属性定义了flex:1，所以：
box-2.height = page-container.height-box-1.height-box-1.margin-top-box-1.margin-bottom-box-2.margin-top-box-2.margin-bottom
- 从(box-2.left,box-2.top)到(box-2.left+root-container.width,box-2.top)
- 从(box-2.left+root-container.width,box-2.top)到(box-2.left+root-container.width,box-2.top+box-2.height)
- 从(box-2.left+root-container.width,box-2.top+box-2.height)到(box-2.left,box-2.top+box-2.height)
- 从(box-2.left,box-2.top+box-2.height)到(box-2.left,box-2.top)

### 布局示例5
```html
/* 这里补充全局样式必选内容 */

.box-1 {
    width: 100px;
    height: 100px;
    border-width: 1px;
    border-style: solid;
    padding: 10px;
}

.box-2 {
    width: 100px;
    flex: 1; /* 这里导致box-2的高度为容器的高度减去box-1的高度和对应的margin */
    border-width: 1px;
    border-style: solid;
    padding: 10px;
}

<body>
    <div class="page-container">
        <div class="box-1"><span>box-1</span></div> <!-- 文字示例 -->
        <div class="box-2"><img src="number-0.svg"/></div> <!-- 图片示例 -->
    </div>
</body>
```

这里需要绘制8根线，具体线条位置参见上文计算方式，然后需要在box-1内绘制文字：
- 文字的左上角坐标是(box-1.padding-left,box-1.padding-top)
同时需要在box-2内绘制图片：
- 图片的左上角坐标是(box-2.padding-left,box-1.height+box-2.padding-top)
- 图片的大小以图片自身的大小为准，运行时直接读取图片不做缩放即可

### 布局示例6
```html
/* 这里补充全局样式必选内容 */

.box-1 {
    width: 100px;
    height: 100px;
    border-width: 1px;
    border-style: solid;
    padding: 10px;
}

.box-2 {
    width: 100px;
    flex: 1; /* 这里导致box-2的高度为容器的高度减去box-1的高度和对应的margin */
    border-width: 1px;
    border-style: solid;
    padding: 10px;
}

<body>
    <div class="page-container">
        <div class="box-1"><span>{placeholder_text}</span></div> <!-- 动态文字示例 -->
        <div class="box-2"><img src="{placeholder_img}"/></div> <!-- 动态图片示例 -->
    </div>
</body>
```

这里需要绘制8根线，具体线条位置参见上文计算方式，然后需要在box-1内绘制文字：
- 文字的左上角坐标是(box-1.padding-left,box-1.padding-top)
同时需要在box-2内绘制图片：
- 图片的左上角坐标是(box-2.padding-left,box-1.height+box-2.padding-top)
- 图片的大小以图片自身的大小为准，运行时直接读取图片不做缩放即可
文字和图片内容需要在运行时动态计算，但是其起始坐标在编译期可以预先计算出来

# 需要处理的input.html（请作为示例看待）
```html
<!DOCTYPE html><html lang="zh-CN"><head><meta charset="UTF-8"><meta name="viewport"content="width=device-width, initial-scale=1.0"><title>800x480信息面板</title><style>*{margin:0;padding:0;box-sizing:border-box}html,body{height:100%;background-color:#FFFFFF}body{display:flex;justify-content:center;align-items:center}@font-face{font-family:'MapleMono';src:url('assets/fonts/MapleMono-NF-CN-Regular.ttf')format('truetype');font-weight:normal;font-style:normal}</style><style>root_container{position:relative;width:800px;height:480px;background-color:#FFFFFF;border:1px solid#000000;font-family:'MapleMono';display:flex;flex-direction:column;padding:10px}time_wrap{display:flex;justify-content:center;align-items:center}time_digit{display:block}date_wrap{text-align:center;font-size:24px;color:#000000;margin:10px 0;display:block}divider{width:calc(100%-10px);height:1px;background-color:#000000;margin:0 auto;display:block}lunar_weather_wrap{display:flex;flex:1;margin:10px 0;position:relative}vertical_divider{position:absolute;left:50%;top:5px;bottom:5px;width:1px;background-color:#000000;transform:translateX(-50%);display:block}lunar_wrap{flex:1;display:flex;flex-direction:column;padding:0 10px}lunar_year{font-size:24px;color:#000000;margin-bottom:5px;display:block;text-align:center}lunar_day{font-size:40px;color:#000000;display:block;text-align:center}lunar_yi_ji{flex:1;display:flex;flex-direction:column;font-size:16px;color:#000000}weather_wrap{flex:1;display:flex;flex-direction:column;padding:0 10px}weather_location_temp_hum_wrap{display:flex;flex-direction:row;justify-content:space-between;padding:0 20px}weather_location{font-size:16px;color:#000000;margin-bottom:5px;display:block}weather_temp_hum{font-size:16px;color:#000000;margin-bottom:5px;display:block}weather_3days{display:flex;justify-content:space-around;flex:1;align-items:center}weather_day{display:flex;flex-direction:column;align-items:center;font-size:16px;color:#000000}weather_icon{width:40px;height:40px;margin:5px 0;display:block}motto_wrap{height:120px;padding:0 20px;display:flex;flex-direction:column;justify-content:center}motto_content{font-size:24px;color:#000000;max-height:96px;width:100%;display:block}motto_source{font-size:16px;color:#000000;margin-top:5px;text-align:right;display:block}network_icon{position:absolute;width:32px;height:32px;top:10px;left:10px;display:block}battery_icon{position:absolute;width:32px;height:32px;top:10px;right:10px;display:block}charging_icon{position:absolute;width:32px;height:32px;top:10px;right:48px;display:block}</style></head><body><div class="root_container"><!--时间--><div class="time_wrap"><img class="time_digit_hour_tens"src="{{time_digit}}"alt=""><img class="time_digit"src="{{time_digit_hour_ones}}"alt=""><img class="time_digit"src="assets/icons/time_digit/digit_colon.svg"alt=":"><img class="time_digit"src="{{time_digit_minute_tens}}"alt=""><img class="time_digits"src="{{time_digit_minute_ones}}"alt=""></div><!--日期--><div class="date_wrap">{{date}}</div><!--水平分割线--><div class="divider"></div><!--农历天气--><div class="lunar_weather_wrap"><div class="vertical_divider"></div><div class="lunar_wrap"><div class="lunar_year">{{lunar_year}}</div><div class="lunar_day">{{lunar_day}}</div><div class="lunar_yi_ji"><div>宜：{{lunar_suitable}}</div><div>忌：{{lunar_avoid}}</div></div></div><div class="weather_wrap"><div class="weather_location_temp_hum_wrap"><div class="weather_location">{{weather_location}}</div><div class="weather_temp_hum">{{weather_temp_hum}}</div></div><div class="weather_3days"><div class="weather_day"><span>{{day1}}</span><img class="weather_icon"src="{{weather_icon1}}"alt=""><span>{{desc1}}</span></div><div class="weather_day"><span>{{day2}}</span><img class="weather_icon"src="{{weather_icon2}}"alt=""><span>{{desc2}}</span></div><div class="weather_day"><span>{{day3}}</span><img class="weather_icon"src="{{weather_icon3}}"alt=""><span>{{desc3}}</span></div></div></div></div><!--水平分割线--><div class="divider"></div><!--格言--><div class="motto_wrap"><div class="motto_content">{{motto_content}}</div><div class="motto_source">{{motto_source}}</div></div><!--状态图标--><img class="network_icon"src="{{network_icon}}"alt="网络状态"><img class="battery_icon"src="{{battery_icon}}"alt="电池状态"><img class="charging_icon"src="{{charging_icon}}"alt="充电状态"></div><script>const mockData={time:"14:35",date:"2025-12-20 星期六",lunar_year:"甲辰龙年闰二月",lunar_day:"初一",lunar_suitable:"出行、祭祀、嫁娶",lunar_avoid:"动土、破土、安葬",weather_location:"北京市",weather_temp_hum:"25℃ 60%RH",weather_3days:[{day:"今天",icon:"sunny",desc:"晴"},{day:"明天",icon:"cloudy",desc:"多云"},{day:"后天",icon:"rain",desc:"小雨"}],motto_content:"路漫漫其修远兮，吾将上下而求索。亦余心之所善兮，虽九死其犹未悔。",motto_source:"——屈原《离骚》",network:"connected",battery:"4",charging:false};function initPanel(){const timeParts=mockData.time.split('');replacePlaceholder('{{time_digit_hour_tens}}',`assets/icons/time_digit/digit_${timeParts[0]}.svg`);replacePlaceholder('{{time_digit_hour_ones}}',`assets/icons/time_digit/digit_${timeParts[1]}.svg`);replacePlaceholder('{{time_digit_minute_tens}}',`assets/icons/time_digit/digit_${timeParts[3]}.svg`);replacePlaceholder('{{time_digit_minute_ones}}',`assets/icons/time_digit/digit_${timeParts[4]}.svg`);replacePlaceholder('{{date}}',mockData.date);replacePlaceholder('{{lunar_year}}',mockData.lunar_year);replacePlaceholder('{{lunar_day}}',mockData.lunar_day);replacePlaceholder('{{lunar_suitable}}',mockData.lunar_suitable);replacePlaceholder('{{lunar_avoid}}',mockData.lunar_avoid);weatherList.forEach((item,index)=>{const dayId=`{{day${index+1}}}`;const descId=`{{desc${index+1}}}`;const iconId=`{{weather_icon${index+1}}}`;replacePlaceholder(dayId,item.day);replacePlaceholder(descId,item.desc);replacePlaceholder(iconId,`assets/icons/weather/${item.icon}.svg`)});replacePlaceholder('{{weather_location}}',mockData.weather_location);replacePlaceholder('{{weather_temp_hum}}',mockData.weather_temp_hum);replacePlaceholder('{{motto_content}}',mockData.motto_content);replacePlaceholder('{{motto_source}}',mockData.motto_source);replacePlaceholder('{{network_icon}}',`assets/icons/network/${mockData.network}.svg`);replacePlaceholder('{{battery_icon}}',`assets/icons/battery/battery-${mockData.battery}.svg`);const chargingIcon=document.getElementById('charging_icon');if(charging){replacePlaceholder('{{charging_icon}}',mockData.charging?'assets/icons/battery/bolt.svg':'none')}else{chargingIcon.style.display='none'}}function replacePlaceholder(placeholder,value){const container=document.getElementById('root_container');container.innerHTML=container.innerHTML.replace(placeholder,value)}function updateTime(){const now=new Date();const hours=now.getHours().toString().padStart(2,'0');const minutes=now.getMinutes().toString().padStart(2,'0');const newTime=`${hours}:${minutes}`;generateTimeIcons(newTime)}window.onload=function(){initPanel();setInterval(updateTime,60000)};</script></body></html>
```

这里只是一个例子，需要parse_html.py可以从中提取出固定的信息，希望python只解析出绘制以下元素的方式：
1. 5个time_digit，在哪里绘制，其大小是多少（来自于svg图片的原始大小），除了图片不同，其位置应当是固定的，不要在生成的python里计算
2. 1个date_wrap，其位置和内容，内容是动态的，但是绘制的位置应当是固定的
3. 1条横向分割线和1条纵向分割线，位置是固定的，且宽度为1
4. lunar_year、lunar_day、lunar_yi_ji、weather_location、weather_temp_hum、3个weather_day（里面的span）和3个weather_icon的内容和显示的位置，他们的内容是动态的（在生成的python计算），但是绘制的位置应当是固定的（解析的python直接写死）
5. motto_content、motto_source的内容和位置，这俩都是动态的，需要在生成的python中根据内容计算位置
一些包装的容器不要直接出现在绘制的python中，他们可以作为计算的辅助常数存在，并且常数的生成应当符合一定规则，如示例中的多个time_digit，其class名称相同，但是生成的常数可以是TIME_DIGIT_1/2/3/4/5_X等
