# mhxx-renkin-reader

MHXX の錬金結果の鑑定画面を写真から読み取って錬金時のフレームを推定するブラウザ向けのツール (作成中)。

- 画像の読み取りは Rust (WebAssembly) で、`renkin-reader/` にある。ビルドとテストは [renkin-reader/README.md](renkin-reader/README.md) を参照

## ライセンス

MIT License です。全文は [LICENSE](LICENSE) を参照してください。

`renkin-reader/assets/` の画像 (ビルド時に埋め込まれる見本) と `renkin-reader/tests/synthetic/` の画像 (テスト用の人工の写真) は
MHXX のゲーム画面から作ったもので、MIT License の対象外です (著作権は株式会社カプコンに帰属します)。

### 使用しているソフトウェア

| | ライセンス | 用途 |
|---|---|---|
| [OpenCV](https://github.com/opencv/opencv) | Apache-2.0 | 画像処理 (位置合わせ・読み取り) のアルゴリズム |
| [apmnnn/mhxx-rng](https://github.com/apmnnn/mhxx-rng) | MIT | 乱数と護石の決まり方 (フレームの探索) |

**OpenCV** の処理を、結果が一致するように Rust で書き直したコード (`renkin-reader/src/vision/cv/` の各ファイル) を含みます。
該当するファイルには冒頭にその旨を記載しています。ライセンス全文と著作権表示は
[THIRD_PARTY_LICENSES/OpenCV](THIRD_PARTY_LICENSES/OpenCV) にあります。

**乱数と護石の決まり方** (`renkin-reader/src/rng.rs`、`renkin-reader/src/rng/`) は apmnnn 氏の mhxx-rng からの移植です。
MIT ライセンスの全文は `renkin-reader/src/rng/xorshift.rs` の冒頭に記載しています。