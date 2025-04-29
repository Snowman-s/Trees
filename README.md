# Trees

Trees は、ブロックプログラミング言語で、以下の特徴を備えています！

- 分かりやすい (easy to understand)
- 読みやすい (readable)
- 曖昧性がない (clear)

```
        ┌─────┐
        │print│
        └───┬─┘
        ┌───┴─┐
    ┌───┤  *  ├──┐
    │   └─────┘  │
┌───┴─┐      ┌───┴─┐
│  3  │      │  4  │
└─────┘      └─────┘ 
```

チュートリアルは https://github.com/Snowman-s/trees/wiki を参照ください。

# ビルド方法

```terminal
$ git clone https://github.com/Snowman-s/Trees.git

$ cd Trees

$ cargo build --release --all
```

上記コマンドを実行すると、`target/release` 内に実行可能ファイルができているはずです。
