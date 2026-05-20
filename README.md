# vim-markdown-composer (Fork)

> **Note:** This is a fork of [euclio/vim-markdown-composer](https://github.com/euclio/vim-markdown-composer).
>
> The main purpose of this fork is to keep a personally maintained version of
> `vim-markdown-composer` usable with my daily Vim workflow. Since the original
> [aurelius](https://github.com/euclio/aurelius) library (which powers the
> Markdown rendering) has not been updated for several years, I created a
> [forked version of aurelius](https://github.com/gaoxiaoliang/aurelius) and
> keep this plugin pointed at that fork.
>
> **What's different:**
> - Uses [gaoxiaoliang/aurelius](https://github.com/gaoxiaoliang/aurelius) instead of the original aurelius
> - Supports Mermaid.js diagrams (flowcharts, sequence diagrams, Gantt charts, etc.)
> - Opens local Markdown links through the live preview instead of serving raw Markdown
> - Keeps relative Markdown links working across multiple local documents
> - Supports browser back/forward navigation between local Markdown preview routes
> - Keeps Vim synchronized with the preview when navigating Markdown links
> - Handles non-ASCII and percent-encoded local Markdown paths in Vim JSON-RPC mode
> - Pinned Rust toolchain version for consistent builds
>
> **References:**
> - [https://blog.rust-lang.org/releases/](https://blog.rust-lang.org/releases/)
> - [https://crates.io/crates/aurelius](https://crates.io/crates/aurelius)

---

![](https://github.com/euclio/vim-markdown-composer/workflows/Continuous%20integration/badge.svg)

vim-markdown-composer is a plugin that adds asynchronous Markdown preview to
[Neovim] and [Vim].

![](https://i.imgur.com/ZtyjjRD.gif)

By default, vim-markdown-composer uses a blazing-fast CommonMark (and
GitHub)-compliant renderer. However, it can be configured to use any external
program for rendering, such as `pandoc`.

## Requirements

This plugin requires Neovim or Vim 8. If you are using an OS with Vim
pre-installed, the system Vim might be too old (see `vim --version`).

This plugin supports Windows, macOS, and Linux.

In addition to Neovim or Vim, vim-markdown-composer requires a distribution of
[Rust] with `cargo`. Check out the [Rust installation guide].

vim-markdown-composer officially targets the latest version of [stable Rust].

## Installation

Use whatever plugin manager you like. If you aren't familiar with plugin
managers, I recommend [vim-plug].

### vim-plug

Here's an example of managing installation with vim-plug:

```vim
function! BuildComposer(info)
  if a:info.status != 'unchanged' || a:info.force
    if has('nvim')
      !cargo build --release --locked
    else
      !cargo build --release --locked --no-default-features --features json-rpc
    endif
  endif
endfunction

Plug 'gaoxiaoliang/vim-markdown-composer', { 'do': function('BuildComposer') }
```

### Vundle

In your `.vimrc`:

```vim
Plugin 'gaoxiaoliang/vim-markdown-composer'
```

Once you have installed the plugin, close Vim/Neovim then (on Linux):

```sh
$ cd ~/.vim/bundle/vim-markdown-composer/
# Vim
$ cargo build --release --no-default-features --features json-rpc
# Neovim
$ cargo build --release
```

### Dein.vim

```
call dein#add('gaoxiaoliang/vim-markdown-composer', { 'build': 'cargo build --release' })
```

### Other plugin managers

You should run `cargo build --release` in the plugin directory after
installation. Vim support requires the `json-rpc` cargo feature.

If you use the above snippet, everything should be taken care of automatically.

## Fork Updates

### Local Markdown navigation

This fork treats local Markdown links as preview routes. When a rendered Markdown
document links to another local `.md`, `.mkd`, or `.markdown` file, clicking that
link keeps the browser in the live preview UI and asks Vim to open the target
file. This means links such as:

```markdown
[Next chapter](chapter-02.md)
[Notes](../notes/index.md)
```

continue to render as live preview pages instead of showing raw Markdown text in
the browser.

The preview URL is based on the target file path, so relative links continue to
work after multiple hops such as `A.md -> B.md -> C.md`.

### Browser history

Because each local Markdown file has its own preview route, normal browser
history is useful again. After navigating between local Markdown files, the
browser back and forward buttons reload the corresponding preview route and keep
both sides synchronized:

- the browser URL
- the rendered preview content
- the current Vim buffer

### Vim synchronization

When a local Markdown link is opened from the preview, Vim receives an
`open_file` message and switches to the target file with `:hide edit`. This keeps
unsaved changes in the previous buffer instead of blocking navigation with Vim's
modified-buffer warning.

For Vim JSON-RPC mode, paths are sent as percent-encoded ASCII and decoded in
Vimscript. This avoids corrupting Chinese or other non-ASCII file names when Vim
is running with a non-UTF-8 internal `encoding`.

### Notes for upgrading

The preview client JavaScript is embedded in the Rust binary through
`gaoxiaoliang/aurelius`, so after pulling updates you should rebuild the plugin:

```sh
# Vim
cargo build --release --locked --no-default-features --features json-rpc

# Neovim
cargo build --release --locked
```

If a browser tab was already open before upgrading, close that preview tab and
run `:ComposerOpen` again so the browser loads the latest embedded client code.

## Plugin Options

By default, `vim-markdown-composer` will open a new browser tab with the rendered preview. This can be prevented by setting the following in your Vim configuration:

```
let g:markdown_composer_autostart = 0
```

## Documentation

`:help markdown-composer`, or check out the `doc` directory.

## Acknowledgments

This plugin is inspired by suan's [vim-instant-markdown].

This plugin was built with [aurelius], a Rust library for live-updating Markdown
previews.

[Rust]: http://www.rust-lang.org/
[cargo]: https://crates.io/
[Neovim]: https://neovim.io/
[Vim]: http://www.vim.org
[vim-instant-markdown]: https://github.com/suan/vim-instant-markdown
[Neovim remote plugin]: https://neovim.io/doc/user/remote_plugin.html
[vim-plug]: https://github.com/junegunn/vim-plug
[msgpack-rpc]: https://github.com/msgpack-rpc/msgpack-rpc
[aurelius]: https://github.com/gaoxiaoliang/aurelius
[stable Rust]: https://www.rust-lang.org/downloads.html
[Rust installation guide]: https://www.rust-lang.org/en-US/install.html
