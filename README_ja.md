# Ordo

> C/C++ のためのモダンなプロジェクトオーケストレーター
>
> ネイティブ開発に Cargo ライクな体験を。

Ordo は Rust 製のビルド・プロジェクト管理ツール。ビルドシステム、依存管理、ツールチェーン、テスト、パッケージング、開発ツールを単一のワークフローに統合する。

## 思想

> エコシステムを置き換えるのではなく、オーケストレーションする。

現代の C/C++ 開発では CMake、Ninja、vcpkg、Conan、pkg-config、clangd、ccache、clang-format、clang-tidy など多数のツールが必要になる。Ordo はこれらのツールに対する統一インターフェースを提供しつつ、既存エコシステムとの互換性を維持する。

## クイックスタート

```bash
ordo new myapp
cd myapp
ordo build
ordo run
```

## 機能

### プロジェクト管理

```bash
ordo new myapp          # 実行ファイルプロジェクト作成
ordo new mylib --lib    # ライブラリプロジェクト作成
ordo init               # 既存ディレクトリを初期化
```

### ビルド

```bash
ordo build              # デバッグビルド
ordo build --release    # リリースビルド
ordo run                # ビルド+実行
ordo check              # 構文チェックのみ（高速）
ordo clean              # ビルド成果物を削除
```

### 依存管理

```toml
# Ordo.toml
[dependencies]
core = { path = "../core" }
fmt = { git = "https://github.com/fmtlib/fmt", tag = "11.1.0" }
spdlog = { version = "1.14", provider = "vcpkg" }
openssl = { provider = "pkg-config" }
zlib = { provider = "system" }
```

```bash
ordo add fmt            # 依存を追加
ordo update             # ロックファイルを更新
ordo tree               # 依存ツリーを表示
```

対応プロバイダー: **vcpkg**、**Conan**、**pkg-config**、**system**、**git**、**Ordo Registry**

### ワークスペース

```toml
[workspace]
members = ["apps/*", "libs/*"]

[workspace.dependencies]
fmt = "11"
```

### テスト

```bash
ordo test               # 全テスト実行
ordo test --filter name # テストをフィルタ
ordo test --jobs 4      # 並列実行
```

GoogleTest、Catch2、doctest を自動検出。

### コード品質

```bash
ordo fmt                # フォーマット（clang-format）
ordo fmt --check        # フォーマット差分チェック（CI用）
ordo lint               # リント（clang-tidy）
ordo lint --fix         # 自動修正
```

### C++ モジュール

C++20 モジュールをファーストクラスサポート:

```toml
[modules]
enabled = true
import-std = true
```

モジュール依存スキャンと BMI 管理を Clang、GCC、MSVC 各コンパイラで自動処理。

### クロスコンパイル

```bash
ordo build --target aarch64-linux-gnu
```

```toml
[target.aarch64-linux-gnu]
compiler = "clang"
sysroot = "/usr/aarch64-linux-gnu"
```

### ビルドプロファイル

```toml
[profile.dev]
opt-level = 0
debug = true
sanitize = ["address", "undefined"]

[profile.release]
opt-level = 3
lto = "thin"
strip = true

[profile.custom]
inherits = "release"
opt-level = "s"
```

### フィーチャーフラグ

```toml
[features]
default = ["logging"]
logging = []
gui = ["dep:qt"]

[dependencies]
qt = { provider = "vcpkg", optional = true }
```

```bash
ordo build --features gui
```

### Watch モード

```bash
ordo watch build
ordo watch test
ordo watch run
```

### IDE 統合

```bash
ordo generate vscode
ordo generate clion
ordo generate clangd
```

`compile_commands.json` はプロジェクトルートに自動生成。

### CMake 互換

```bash
ordo import cmake       # CMakeLists.txt → Ordo.toml（移行支援）
ordo generate cmake     # Ordo.toml → CMakeLists.txt
ordo generate presets   # CMakePresets.json 生成
```

### CI

```bash
ordo ci                 # CI パイプライン一括実行
ordo generate github-actions
ordo generate gitlab-ci
```

### パッケージング

```bash
ordo install            # システムにインストール（pkg-config + CMake config 自動生成）
ordo package            # 配布用アーカイブ作成
ordo publish            # Ordo Registry に公開
```

### 診断

```bash
ordo doctor             # 開発環境チェック
ordo config show        # 解決済み設定を表示
ordo config show --origin  # 各値の出典を表示
```

## ビルドバックエンド

Ordo は Ninja ビルドファイルを直接生成する（CMake はパイプラインに含まない）。これにより C++ モジュール、依存スキャン、ビルド最適化を完全に制御しつつ、Ninja の実績あるインクリメンタルビルドと並列処理を活用できる。

## 設定

プロジェクト設定は `Ordo.toml` に記述:

```toml
[package]
name = "myapp"
version = "0.1.0"
type = "executable"

[language]
cpp = "c++20"

[toolchain]
compiler = "clang"
linker = "lld"

[cache]
tool = "auto"  # sccache > ccache > none
```

## ライセンス

以下のいずれかを選択して利用可能:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
