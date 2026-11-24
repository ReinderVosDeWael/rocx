# Contributing

With Cargo and `rustfmt` installed, enable the optional formatting hook once per clone:

```sh
uv tool install prek
prek install
```

[prek](https://prek.j178.dev/) checks formatting before each commit and temporarily sets aside unstaged changes to tracked files. If the check fails, run `cargo fmt` and stage the intended changes. CI also checks formatting.

[Tests](tests/README.md) · [Documentation builds](docs/DOCUMENTATION.md) · [Module map](docs/MODULES.md) · [OOXML reference](docs/OOXML.md) · [Release guide](docs/RELEASING.md)
