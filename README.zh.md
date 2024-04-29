# sūshì

sūshì是一个简单，但是便于自定义的静态网站生成器 ／ 博客生成器。

## 安装

### 使用Cargo安装（推荐）

```
cargo install sushi-gen
```

命令行工具`ssushi`将安装在`.cargo/bin`。接下来检查安装是否成功：

```
ssushi --help
```

### 从源码编译

首先克隆〔clone〕本仓库，然后进入其文件夹

```
git clone https://github.com/fpg2012/sushi
cd susho
```

然后使用Cargo编译：

```
cargo build --release
```

二进制可执行文件`ssushi`将在编译完成后出现在`target/release`中。

## 快速上手

以下内容暂只针对Linux用户：

1. 安装sushi

2. 创建一个新的文件夹并进入（假设文件夹名为`blog`）

3. 新建名为`_site.yml`的文件，内容如下

   ```yaml
   site_name: "My Blog"
   author: "my name"
   url: "//127.0.0.1:5000" # 站点首页的url
   theme_dir: "_theme"
   ```

4. 下载主题，如[empty](https://github.com/fpg2012/sushi-theme-empty)

   ```
   git clone https://github.com/fpg2012/sushi-theme-empty _theme
   ```

5. 新建名为`posts`的文件夹并往里面添加markdown文件，比如新建`helloworld.md`
   
   ```md
   ---
   layout: post
   title: "Hello World"
   date: 2024-01-01
   description: "My first post."
   category: [hello]
   tag: [blog, sushi, ssg]
   ---

   # Hello World

   helloworld

   $E = mc^2$
   ```

6. 构建，运行

   ```
   ssushi build
   ```

7. 构建完成后，生成好的站点放在`_gen`之中。可以安装一个`sfz`将其部署到本地，以便查看效果。
   
   ```
   cargo install sfz
   sfz -r _gen
   ```

## Sūshì是如何工作的？

### 站点结构

一个sushi站点看起来也许会像这样

```
sushi-theme-letter
├── assets
├── _converters (*)
│   ├── convert.sh
│   └── pandoc-katex
├── _gen
├── _includes (*)
│   ├── footer.liquid
│   ├── header.liquid
│   └── katexcss.liquid
├── index.md
├── notes
├── posts
│   ├── 2021-04-04-some-post-with-assets
│   │   ├── pic1.png
│   │   ├── pic2.png
│   │   └── index.md
│   └── 2022-03-18-some-post.md
├── _site.yml (*)
└── _templates (*)
    ├── page.liquid
    └── post.liquid
```

实际上只有`_converters`、`_includes`、`_templates`、`_site.yml`是必须的（当然也**不能**改名）。sushi启动之后，首先会读取这些文件夹和配置文件，将其中的内容载入到内存中。

模板（使用liquid模板语言编写）和片段（也用liquid编写）分别放在`_templates`和`_includes`文件夹中。站点配置写在`_site.yml`中。`_converters`文件夹存储页面转换程序（比如markdown解析器）。

> 注意，sushi**不直接解析**markdown（或者其他页面格式），而是调用用户提供并指定的转换程序。所谓转换，即将页面类型转换成HTML，然后插入到模板中。转换器由用户提供，因此你可以自己写转换器提供给sushi，或者写一点脚本调用一些通用的程序（比方说**pandoc**）。

在读取这些最重要的配置之后，sushi使用转换程序对所有页面进行处理。以`.`或`_`开头的文件/文件夹将会被忽略。另外，只有用户指定的页面文件类型会参与转换，其余类型的文件将被直接复制。

生成后的站点将放置在`_gen`文件夹中。

### 站点配置

`_site.yml`的内容可能如下：

```yaml
site_name: "my site"
author: "my name"
url: "https://example.com"
# ...
convert_ext: ["html", "md"]
converter_choice:
  - md: "converter.sh"
taxonomies: ["category", "tag"]
```

| 配置                 | 类型        | 作用                              |
| ------------------ | --------- | ------------------------------- |
| `converter_ext`    | 字符串数组     | 页面文件的有效后缀名。列在此的后缀名将被认为是页面文件的后缀名 |
| `converter_choice` | 映射〔map〕数组 | 指定每种后缀名使用哪一种转换器                 |
| `taxonomies`       | 映射〔map〕数组 | 分类方式列表                          |
| `url`              | 字符串       | 站点的url。如果不填，将使用`/`              |
| `theme_dir` | string | 主题文件夹 |
| `gen_dir` | string | 放置生成的文件的文件夹. default: `_gen` |
| `converter_dir` | string | converters文件夹. default: `_converters` |
| `templates_dir` | string | templates文件夹. default: `_templates` |
| `includes_dir` | string | 放置liquid partials的文件夹. default: `_includes` |

### 页面扉页

扉页〔front matter〕包含关于页面的配置

```
---
layout: post
title: "Test Sushi"
date: "2022-03-12"
tag: ["a", "b", "c"]
category: ["dev"]
---
```

| 配置名                | 作用                    |
| ------------------ | --------------------- |
| `layout`           | （必填）使用的模板名            |
| `date`             | （必填）日期，类似“2022-03-12” |
| `[分类方式名]`          | 类别列表                  |
| `paginate`         | 用于分页的列表               |
| `paginate_batches` | 每页的项目数                |
| `next`             | 下一页的id                |
| `last`             | 上一页的id                |

### 编写模板

#### Liquid

sushi使用了liquid模板语言的Rust实现。关于Liquid的语法，请参阅liquid模板语言和其Rust实现的文档（使用的「板条箱」〔crate〕为liquid）。

#### 全局对象

sushi提供了以下全局对象：

| 对象名          | 作用                                                                            |
| ------------ | ----------------------------------------------------------------------------- |
| `site`       | 所有`_site.yml`中的配置将被插入到这个对象中。比方说，`site.site_name`就是`_site.yml`中设置的`site_name`。 |
| `page`       | 本页的扉页信息将插入到此对象中                                                               |
| `content`    | 本页的内容                                                                         |
| `sitetree`   | 站点树                                                                           |
| `taxo`       | 分类方式列表                                                                        |
| `id_to_page` | 将页面id映射为页面对象                                                                  |
| `all_pages`  | 所有页面id的列表                                                                     |
| `paginator`  | 分页器                                                                           |

另外，除了`_site.yml`中定义的键值对和扉页信息，`site`和`page`对象还包含了一些生成的信息

| 名           | 作用      |
| ----------- | ------- |
| `site.time` | 生成日期    |
| `page.url`  | 页面URL   |
| `page.path` | 原始页面的路径 |
| `page.next` | 下一页的id  |
| `page.last` | 上一页的id  |
| `page.content` | 页面的原始内容 |
| `page.search_text` | 一般和`page.content`一样 |

> `page.search_text`本意是用于实现搜索功能

`sitetree`对象

| 名                        | 作用                                                                                                                                               |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `sitetree._home`         | 站点根目录对应的对象                                                                                                                                       |
| `sitetree.[文件夹]`         | `[文件夹]`对应的对象                                                                                                                                     |
| `sitetree.[文件夹1].[文件夹2]` | `[文件夹1]/[文件夹2]`对应的对象                                                                                                                             |
| `sitetree.[文件夹]._list`   | 文件夹中所有子页面的id（如果子文件夹有索引页，那么索引页的id也会被列进来）。比方说，`posts`文件夹中的所有页面的id列在`sitetree.posts._list`中。同样，`posts/notes`中的所有页面将列在`sitetree.posts.notes._list`中。 |

`taxo`对象

| 名                                       | 作用                                                                                                    |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `taxo._key`                             | 分类方式列表                                                                                                |
| `taxo.[taxonomy]`                       | `taxonomy`对应的对象，比如说`taxo.tag`, `taxo.category`                                                        |
| `taxo.[taxonomy].[taxonomy_value]`      | 按照分类方式`taxonomy`，属于`taxonomy_value`这一类的所有页面的`page_id`列表，比如说，所有包含`rust`这种`tag`的页面的id将列入`taxo.tag.rust` |
| `taxo.[taxonomy].[taxonomy_value]._key` | 所有出现过的分类                                                                                              |

#### 模板的扉页

| 名        | 作用   |
| -------- | ---- |
| `layout` | 父模板名 |

sushi支持模板继承，方法类似Jekyll。如果`post`继承`page`，那么`post`的渲染结果将暂存，然后作为`page`模板的`content`进行二次渲染。

```
page_content =="post"=> result1
result1 =="page"=> result2 // the final result in this example
```

#### 添加模板片段

如果某个代码片段需要用到多个模板中，那么建议将它独立出来，作为一个“模板片段”，放置在`_includes`文件夹中。其他模板可以引用之。

例如，如果`_includes`文件夹中有`header.liquid`，那么你可以在模板中使用`{{ include header }}`把这个片段插入进来。

### 分页器

分页器，顾名思义，用于分页。由于一些限制，sushi分页器的使用略有些复杂。

首先，sushi的分页器基于**列表**，列表是分页的依据。分页器将列表分成多份，然后对每一份都渲染一个页面。总之，要用分类器，需要像下面这样在**页面扉页**中提供一个列表

```yaml
---
#...
paginate: sitetree.posts._list # 要用于切分的列表
paginate_batches: 4 # 每份的元素个数
---
```

然后，在你的**模板**中使用`paginator`对象。比如：

```liquid
{% for page_id in paginator.current_batch %}
  <li><a href="{{ id_to_page[page_id].url }}">{{ id_to_page[page_id].title }}</a></li>
{% endfor %}
{% if paginator.batch_num > 1 %} <!--more than one page-->
{% if paginator.next_batch_num %}
  <a href="{{ paginator.batch_urls[paginator.next_batch_num] }}">{{ paginator.next_batch_num }}</a>
{% endif %}
{% if pageinator.last_batch_num %}
  <a href="{{ paginator.batch_urls[paginator.last_batch_num] }}">{{ paginator.last_batch_num }}</a>
{% endif %}
{% endif %}
```

假设我们要分的这个页面是`test.md`，那么在生成的网站中，页面变成下面这种结构

```
test.html
test
├─1.html
├─2.html
├─ ....
└─10.html
```

| 名                             | 作用           |
| ----------------------------- | ------------ |
| `paginator.current_batch`     | 当前的这一批元素的列表  |
| `paginator.current_batch_num` | 当前批的索引       |
| `paginator.next_batch_num`    | 下一批的索引       |
| `paginator.last_batch_num`    | 上一批的索引       |
| `paginator.batch_urls`        | 每一批对应的页面的url |
| `paginator.items`             | 用于分页的原数组     |
| `paginator.batch_num`         | 批数           |

### 编写转换器

所谓转换器，是一个从标准输入读取输入，输出到标准输出的可执行程序，通常是把markdown或者其他格式转换成html。这种设计使转换器的编写相当灵活，也相当简单。比方说你可以写个调用pandoc的bash脚本：

```
#!/bin/bash
pandoc -f [filter] --katex
```

修改权限使之可执行，然后放到`_converter`中，然后在站点配置中让sushi调用它。

### 主题

如果需要使用主题，建议将主题放置在`_theme`文件夹中，并在`_site.yml`中设置：

```yaml
theme_dir: "_theme"
```

如有必要，可以主题文件夹命名为任意下划线`_`开头的名称，`_site.yml`中的`theme_dir`也要做对应修改。以下内容假设主题文件夹名为`_theme`

主题文件夹的结构和站点结构相同。在生成站点时，sushi会将主题文件夹中的站点树与现有的站点树合并，也会将`_theme/_site.yml`中的配置合并进来。若站点树与主题的站点树存在冲突，则会忽略主题中冲突的部分。同理，`_site.yml`中的配置会覆盖`_theme/_site.yml`中的配置。

例如，假设站点结构如下

```
site
├── assets
│   └── a.jpg
├── helloworld.md
├── _site.yml
└── _theme
    ├── assets
    │   └── favicon.png
    ├── _converters
    ├── _includes
    ├── _site.yml
    └── _templates
```

则生成之后的站点树如下

```
_gen
├── assets
│   ├── a.jpg
│   └── favicon.png
└── helloworld.html
```

### 站点初始化（不推荐）

当你执行`ssushi init [sitename]`的时候，sushi会自动搜寻名为`default`的站点模板文件夹，然后把那个文件夹复制到工作目录（并重命名为`sitename`）。sushi搜寻的文件夹包括项目配置文件夹和当前工作目录。

你可以使用`--theme [starter_name/starter_path]`来使用其他的站点模板。

注意，sushi并不自带默认模板，必须自己创建/下载一个。

### `ssushi`的命令行参数

`ssushi`：

```
Usage: ssushi [OPTIONS] <COMMAND>

Commands:
  init   
  build  
  help   Print this message or the help of the given subcommand(s)

Options:
      --debug    
  -q, --quiet    
  -h, --help     帮助信息
  -V, --version  显示版本号
```

`ssushi build`：

```
Usage: ssushi build [OPTIONS]

Options:
  -A, --regen-all                
  -c, --config <CONFIG>          [default: _site.yml]
  -g, --gen <GEN>                
      --includes <INCLUDES>      
      --converters <CONVERTERS>  
      --templates <TEMPLATES>    
      --theme <THEME>            
  -s, --subpath <SUBPATH>        手动指定需要生成的目录/文件
      --naive-skip               启用naive-skip（仅通过时间戳判断是否生成）
  -h, --help                     帮助信息
```

### sushi的设计目的和“最佳”实践

参见 [如何更好地使用sushi](https://nth233.top/posts/2022-12-29-%E5%A6%82%E4%BD%95%E6%9B%B4%E5%A5%BD%E5%9C%B0%E4%BD%BF%E7%94%A8sushi.html)
