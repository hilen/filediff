# filediff

The diff of one file as plain data: its hunks and lines, the two sides of an
image, and what it was taken against. Also the patches that take one hunk or
one line back out of the working tree, for `git apply -R`.

It has no UI and runs no git. The diff view of [hilen](https://github.com/hilen/hilen),
`DiffPanel` of its `diff-view` feature, draws these types. A program with no window,
like a daemon that runs git, builds them and sends them as JSON.

## Why a repo of its own

A git dependency is one crate per commit of its repo. The view in hilen and the
code that builds a diff have to agree on 1 copy of these types. Inside the hilen
repo every hilen commit would make a new copy, so every app pin of hilen would
have to match the pin of each crate that builds diffs. Here the commit moves only
when the types change, and that is rare.

```toml
filediff = { git = "https://github.com/hilen/filediff", rev = "<full sha>" }
```

Every crate that names `filediff` must pin the same rev, hilen included.
