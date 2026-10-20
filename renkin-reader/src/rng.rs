//! 乱数とフレームの探索: お守りが出る錬金のフレームを探す (mhxx-rng の移植。マカフシギ錬金)。
//!
//! 今は未実装で、いつも空を返す

use crate::types::{Charm, InputError};

/// [start, end) のフレームのうち、charm が (index があればその何個目に) 出るものを返す
pub fn search_frames(
    charm: &Charm,
    index: Option<u8>,
    start: u32,
    end: u32,
) -> Result<Vec<u32>, InputError> {
    if start > end {
        return Err(InputError::FrameRange { start, end });
    }
    let _ = (charm, index);
    // TODO: mhxx-rng の乱数とお守りの生成 (マカフシギ錬金)
    Ok(Vec::new())
}
