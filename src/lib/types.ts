// 読み取りとフレーム探索の型。wasm (renkin-reader/pkg、wasm-pack が Rust の型から書き出す) の型定義をそのまま使う
export type {
  Candidate,
  Charm,
  Field,
  Fields,
  FrameMatch,
  PointRange,
  ReadResult,
  RejectReason,
  SearchCharm,
  SkillInfo,
} from '../../renkin-reader/pkg/renkin_reader.js'
