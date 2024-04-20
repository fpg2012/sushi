[中文README](https://github.com/fpg2012/sushi/blob/main/README.zh.md)

# sūshì

Sūshì is a simple but customizable static site generator / blog generator written in Rust.

## Installation

### Install with Cargo (Recommended)

```
cargo install sushi-gen
```

`ssushi` will be installed in `.cargo/bin`. Now check the installation.

```
ssushi --help
```

### Compile from Source Manually

Clone this repository and `cd` into it:

```
git clone https://github.com/fpg2012/sushi
cd sushi
```

And build it with Cargo:

```
cargo build --release
```

The binary executable `ssushi` will be placed in `target/release`

## Quick Start

For Linux users:

1. Install sushi

2. Create a new directory for your site which can be named `blog`. `cd` into this directory.

3. Get a theme (for example, [empty](https://github.com/fpg2012/sushi-theme-empty))
   
   ```
   git clone https://github.com/fpg2012/sushi-theme-empty _theme
   ```

3. Create `_site.yml`
   
   ```yaml
   site_name: "My Blog"
   author: "my name"
   url: "//127.0.0.1:5000" # site url
   theme_dir: "_theme"
   ```

4. Create a post. Create a directory named `post`, and then put some markdown files into it, for example, `posts/helloworld.md`

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

4. Build the site
   
   ```
   ssushi build
   ```

5. Now the site is generated into `_gen` folder. You can install `sfz` to serve the site.
   
   ```
   cargo install sfz
   sfz -r _gen
   ```

## How does sūshì work?

### Site Structure

A Sūshì site might look like this:

```
sushi-theme-letter
├── assets
├── _converters (*)
│   ├── convert.sh
│   └── pandoc-katex
├── _gen
├── _includes (*)
│   ├── footer.liquid
│   ├── header.liquid
│   └── katexcss.liquid
├── index.md
├── notes
├── posts
│   ├── 2021-04-04-some-post-with-assets
│   │   ├── pic1.png
│   │   ├── pic2.png
│   │   └── index.md
│   └── 2022-03-18-some-post.md
├── _site.yml (*)
└── _templates (*)
    ├── page.liquid
    └── post.liquid
```

Actually only `_converters`, `_includes`, `_templates` and `_site.yml` are necessary and should NOT be renamed.  Once sushi starts, it reads these files and folders first and load them into memory.

Templates (written in liquid template language) and partials (in liquid too) should be stored in `_templates` and `_includes` respectively. Site configurations are written in `_site.yml`. `_converters` stores executables for converting page files into HTML pages (i.e. markdown parsers).

> Note that sūshì does not parse markdown (or any other format) directly, what it does is simply compiling templates you provide and insert the converted page contents into them. You are required to write converters which are basically simple scripts to execute **pandoc** or any other parser you like, for sushi to invoke.

After reading these important configurations, sushi convert all pages by executing the converters. Folders and file start with `.` or `_` will be ignored. All file that are not recognized as "page files" will be copied directly to the corresponding locations.

Generated site will be put into `_gen` folder.

### Site Configuration

`_site.yml` might look like this:

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

| configuration      | value type      | function                                                                                           |
| ------------------ | --------------- | -------------------------------------------------------------------------------------------------- |
| `convert_ext`      | array of string | Valid extensions of page file. File with extension listed here is considered as page file.         |
| `converter_choice` | array of map    | Specific which converter to be used. If not set, all pages will be inserted to templates directly. |
| `taxonomies`       | array of map    | List of taxonomies                                                                                 |
| `url`              | string          | Base url of the site. If not set, `"/"` will be used.                                              |
| `theme_dir` | string | Theme directory |
| `gen_dir` | string | folder for the generated files. default: `_gen` |
| `converter_dir` | string | folder of converters. default: `_converters` |
| `templates_dir` | string | folder of liquid templates. default: `_templates` |
| `includes_dir` | string | folder of liquid partials. default: `_includes` |


### Page Front Matter

Front matter contains the configuration of the page.

```
---
layout: post
title: "Test Sushi"
date: "2022-03-12"
tag: ["a", "b", "c"]
category: ["dev"]
---
```

| name               | usage                              |
| ------------------ | ---------------------------------- |
| `layout`           | (required) name of the template    |
| `date`             | (required) date, like "2022-03-12" |
| `[taxonomy name]`  | list of taxonomy value             |
| `paginate`         | the list used for pagination       |
| `paginate_batches` | number of items in a batch         |
| `next`             | id of next page                    |
| `last`             | id of last page                    |

### Write Templates

#### Liquid

Sushi uses the rust implementation of liquid template language. For syntax of liquid, please refer to the documentation of liquid languague and liquid crate.

#### Global Objects

Sushi offers some global liquid variables.

| name         | usage                                                                                                                                                                                    |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `site`       | All configurations in `_site.yml` are inserted into the object. For example, `site.site_name` is the `site_name` set in `_site.yml`. `site.time` is the datetime of generating the site. |
| `page`       | Front matter of current page.                                                                                                                                                            |
| `content`    | Content of current page. string.                                                                                                                                                         |
| `sitetree`   | Site tree                                                                                                                                                                                |
| `taxo`       | Taxonomy list                                                                                                                                                                            |
| `id_to_page` | Map page_id to page object                                                                                                                                                               |
| `all_pages`  | List of all page_id                                                                                                                                                                      |
| `paginator`  | Paginator                                                                                                                                                                                |

Besides the key-value pair defined by user in `_config.yml` and front matter, `site` and `page` object contains some generated information.

| name        | usage                           |
| ----------- | ------------------------------- |
| `site.time` | Datetime of generating the site |
| `page.url`  | URL of page                     |
| `page.path` | Path of original page file      |
| `page.next` | ID of next page                 |
| `page.last` | ID of last page                 |
| `page.content` | original content of the page |
| `page.search_text` | usually same as `page.content` |

> `page.search_text` is meant to used for implementing search functionality

`sitetree` object

| name                           | usage                                                                                                                                                                                                                                                       |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `sitetree._home`               | Object of root directory                                                                                                                                                                                                                                    |
| `sitetree.[folder]`            | Object of `[folder]` directory                                                                                                                                                                                                                              |
| `sitetree.[folder1].[folder2]` | Object of `folder1/folder2`                                                                                                                                                                                                                                 |
| `sitetree.[folder]._list`      | page_ids of pages in the folder. Index page_id of child folder will be listed here too. For example, all pages in "post" folder will be list in `sitetree.post._list`. Similarly, all pages in "posts/notes" will be listed in `sitetree.posts.notes._list` |

`taxo` object

| name                                    | usage                                                                                                                                 |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `taxo._key`                             | List of taxnonomies.                                                                                                                  |
| `taxo.[taxonomy]`                       | Object of `taxonomy`, for exmaple `taxo.tag`, `taxo.category`                                                                         |
| `taxo.[taxonomy].[taxonomy_value]`      | List of page_id of pages with the taxonomy value. For example, all page_id of pages with tag "rust" will be listed in `taxo.tag.rust` |
| `taxo.[taxonomy].[taxonomy_value]._key` | List of valid taxonomy value                                                                                                          |

#### Template Front Matter

| name     | usage                            |
| -------- | -------------------------------- |
| `layout` | template name of parent template |

Template inheritance is supported, which is similar to that of Jekyll. If template `post` inherites template `page`, the render result of `post` will be the `content` inserted to `page`.

```
page_content =="post"=> result1
result1 =="page"=> result2 // the final result in this example
```

#### Add Partials

If a snippet of code is used by multiple templates, it is recommended to split them to a partial file. All partial file should be put in `_includes` folder.

For example, if `header.liquid` is put in `_includes` folder, you can use `{{ include header }}` in your template to include it.

### Paginator

Paginator is used to split a page into mutiple pages (for example, when showing a super long list of page titles in home page).

Usage of paginator is a little bit complex.

First, paginator of sushi is based on "list", it splits the list into multiple "batches". So you should put the list you want to split into **page** front matter. 

```yaml
---
#...
paginate: sitetree.posts._list # the list you want to split
paginate_batches: 4 # the number of item in a batch
---
```

And then, use the `paginator` object in your **template**. For example: 

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

After pagination, page `test.md` might be split into

```
test.html
test
├─1.html
├─2.html
├─ ....
└─10.html
```

| name                          | usage                     |
| ----------------------------- | ------------------------- |
| `paginator.current_batch`     | current batch             |
| `paginator.current_batch_num` | index of current batch    |
| `paginator.next_batch_num`    | index of next batch       |
| `paginator.last_batch_num`    | index of  last batch      |
| `paginator.batch_urls`        | list of batch urls        |
| `paginator.items`             | the list before splitting |
| `paginator.batch_num`         | number of batches         |

### Write Converters

Writing converters is quite simple. A converter is a executable reads input from stdin and writes output to stdout. That's all.

For example, you can write a shell script to execute pandoc

```
#!/bin/bash
pandoc -f [filter] --katex
```

### Themes

If you need to use a theme, it is recommended to put it into `_theme`, and add a line in `_site.yml`:

```yaml
theme_dir: "_theme"
```

It is actually possible to use any name started with an underscore `_` to name the theme directory. Below we assume you stick to the recommended name `_theme`.

The theme folder structure is the same as the site structure. When generating a site, sushi will merge the site tree in the theme folder with the existing site tree, and will also merge the configuration in `_theme/_site.yml`. If the site tree conflicts with the site tree of a theme, the conflicting parts of the theme are ignored. In the same way, the configuration in `_site.yml` will override the configuration in `_theme/_site.yml`.

For example, assume the site structure is as follows

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

Then, the generated site will be

```
_gen
├── assets
│   ├── a.jpg
│   └── favicon.png
└── helloworld.html
```

### Site Initialization and Starters (not recommended)

When you execute `ssushi init [sitename]`, sushi will search for a starter named "default" in project config folder and current working directory, and then simply copy it to `./[sitename]`.

You can use `--theme [starter_name/starter_path]` option to use other starters.

Note that there is no default starter after installation with Cargo, you should create one manually.

### Why did I make sushi?

Refer to [my blog post](https://nth233.top/posts/2022-12-29-%E5%A6%82%E4%BD%95%E6%9B%B4%E5%A5%BD%E5%9C%B0%E4%BD%BF%E7%94%A8sushi.html) (in Chinese)