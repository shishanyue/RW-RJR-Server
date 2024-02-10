# 贡献

## 预先准备

你需要事先准备好Rust基本开发环境和工具链。

### 手动安装

前往[Rust语言官网](https://www.rust-lang.org/)可以获取`rustup`程序并自动安装基本工具链。

### 通过包管理器安装

Windows：

```sh
winget install rustup
```
ArchLinux：

```sh
pacman -S rustup
```

## 选取IDE或编辑器

推荐[VSCode](https://code.visualstudio.com/)并安装[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)插件。

## Linting和格式化

使用`cargo clippy`进行Linting，使用`cargo fmt`进行代码格式化。

## 构建项目

使用`cargo build`来构建整个项目。

如果你需要构建并运行，使用`cargo run`。

## 贡献代码到本仓库

贡献代码前建议新建相关issue，之后在代码完成后发送PR到本仓库。