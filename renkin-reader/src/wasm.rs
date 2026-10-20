//! ブラウザ (TypeScript) から呼ぶ入口。本体を呼んで、結果を JS のオブジェクトに変えるだけ

use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::types::{Charm, ReadResult, SkillId};

#[wasm_bindgen(js_name = Reader)]
pub struct WasmReader(crate::Reader);

#[wasm_bindgen(js_class = Reader)]
impl WasmReader {
    /// 重い準備を一度だけやる
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmReader {
        WasmReader(crate::Reader::new())
    }

    /// RGBA の画素 (ImageData.data など。width x height x 4 バイト) を読む
    pub fn read(&mut self, rgba: &[u8], width: u32, height: u32) -> Result<ReadResult, JsError> {
        Ok(self.0.read(rgba, width, height)?)
    }

    /// 準備 (new Reader) の段階ごとの時間
    #[wasm_bindgen(js_name = initTimings, unchecked_return_type = "Timing[]")]
    pub fn init_timings(&self) -> Result<JsValue, JsError> {
        Ok(serde_wasm_bindgen::to_value(self.0.init_timings())?)
    }

    /// 直前に読んだ画像の、位置合わせ後の画面 (1280 x 720 の RGBA)。位置合わせに失敗していれば undefined
    #[wasm_bindgen(js_name = alignedImage)]
    pub fn aligned_image(&self) -> Option<Vec<u8>> {
        self.0.aligned_image().map(<[u8]>::to_vec)
    }
}

#[derive(Serialize, Tsify)]
pub struct SkillEntry {
    pub id: SkillId,
    pub name: &'static str,
}

/// マカフシギ錬金で出るスキルの番号と表示名 (番号の小さい順)
#[wasm_bindgen(unchecked_return_type = "SkillEntry[]")]
pub fn skills() -> Result<JsValue, JsError> {
    let list: Vec<SkillEntry> = crate::skills()
        .iter()
        .map(|&(id, name)| SkillEntry { id, name })
        .collect();
    Ok(serde_wasm_bindgen::to_value(&list)?)
}

/// [start, end) のフレームのうち、charm が (index があればその何個目に) 出るもの
#[wasm_bindgen(js_name = searchFrames)]
pub fn search_frames(
    charm: Charm,
    index: Option<u8>,
    start: u32,
    end: u32,
) -> Result<Vec<u32>, JsError> {
    Ok(crate::search_frames(&charm, index, start, end)?)
}
