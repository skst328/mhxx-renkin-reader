"""renkin-reader のテストで使う、比べるためのデータ (tests/fixtures/) を作る.

OpenCV の結果を正解として書き出す。OpenCV の入った Python で動かす (experimental-charm-reader の venv など)。

usage: python tools/make_fixtures.py
"""
import json
import sys
from pathlib import Path

import cv2
import numpy as np

FIXTURES = Path(__file__).resolve().parents[1] / "renkin-reader" / "tests" / "fixtures"
EXPERIMENTAL = Path(__file__).resolve().parents[2] / "experimental-charm-reader"  # Python 版 (正解を出す側)


def to_rgba(img):
    """cv2.imread (IMREAD_UNCHANGED) の結果を RGBA に並べ直す. 透明度が無ければ 255."""
    if img.ndim == 2:
        return np.dstack([img, img, img, np.full_like(img, 255)])
    if img.shape[2] == 3:
        return np.dstack([img[..., ::-1], np.full(img.shape[:2], 255, img.dtype)])
    return img[..., [2, 1, 0, 3]]


def load():
    """PNG の読み込みの確認用: RGBA・RGB・灰色の小さな PNG と、OpenCV で読んだ画素."""
    out = FIXTURES / "load"
    out.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(0)
    h, w = 5, 7  # 幅と高さを違う値にして、縦横の取り違えを見つける
    images = {
        "rgba.png": rng.integers(0, 256, (h, w, 4), dtype=np.uint8),  # OpenCV の並びは BGRA
        "rgb.png": rng.integers(0, 256, (h, w, 3), dtype=np.uint8),   # BGR
        "gray.png": rng.integers(0, 256, (h, w), dtype=np.uint8),
    }
    expected = {}
    for name, img in images.items():
        cv2.imwrite(str(out / name), img)
        rgba = to_rgba(cv2.imread(str(out / name), cv2.IMREAD_UNCHANGED))
        expected[name] = {"width": w, "height": h, "rgba": rgba.reshape(-1).tolist()}
    (out / "expected.json").write_text(json.dumps(expected) + "\n")


def _save(name, data):
    out = FIXTURES / "cv"
    out.mkdir(parents=True, exist_ok=True)
    (out / f"{name}.json").write_text(json.dumps(data) + "\n")


def _img(a):
    """numpy の 2 次元配列を {"width", "height", "data"} にする."""
    return {"width": int(a.shape[1]), "height": int(a.shape[0]), "data": a.reshape(-1).tolist()}


def color():
    """色チャンネルの最大値と、HSV (OpenCV の 8 ビット: H は 0〜179)."""
    rng = np.random.default_rng(1)
    bgr = rng.integers(0, 256, (9, 13, 3), dtype=np.uint8)
    # 灰色・純色・黄色・同じ値の組み合わせなど、割り算の境目になりやすい色を足す
    special = [(0, 0, 0), (255, 255, 255), (128, 128, 128), (0, 0, 255), (0, 255, 0), (255, 0, 0),
               (0, 255, 255), (40, 200, 230), (10, 180, 200), (255, 0, 255), (1, 0, 0), (0, 1, 1)]
    bgr[0, :len(special)] = special
    rgba = np.dstack([bgr[..., ::-1], np.full(bgr.shape[:2], 255, np.uint8)])
    hsv = cv2.cvtColor(bgr, cv2.COLOR_BGR2HSV)
    _save("color", {"rgba": {"width": int(bgr.shape[1]), "height": int(bgr.shape[0]), "data": rgba.reshape(-1).tolist()},
                    "max": _img(bgr.max(axis=2)), "h": _img(hsv[..., 0]), "s": _img(hsv[..., 1]), "v": _img(hsv[..., 2]),
                    "yellow": _img(cv2.inRange(hsv, (18, 120, 150), (35, 255, 255)))})


def morph():
    """矩形の収縮・膨張・トップハット (画像より大きい 25x25 も)."""
    rng = np.random.default_rng(2)
    g = rng.integers(0, 256, (20, 30), dtype=np.uint8)
    out = {"src": _img(g)}
    for k in (5, 25):
        ker = cv2.getStructuringElement(cv2.MORPH_RECT, (k, k))
        out[f"erode{k}"] = _img(cv2.erode(g, ker))
        out[f"dilate{k}"] = _img(cv2.dilate(g, ker))
        out[f"tophat{k}"] = _img(cv2.morphologyEx(g, cv2.MORPH_TOPHAT, ker))
    _save("morph", out)


def resize():
    """線形補間の拡大縮小 (cv2.resize(fx=fy=s), 見本の倍率と同じ)."""
    rng = np.random.default_rng(3)
    g = rng.integers(0, 256, (21, 100), dtype=np.uint8)  # スキル名の見本くらいの大きさ
    out = {"src": _img(g)}
    for s in (0.97, 1.03, 0.5, 2.0):
        out[str(s)] = _img(cv2.resize(g, None, fx=s, fy=s, interpolation=cv2.INTER_LINEAR))
    _save("resize", out)


def matching():
    """照合 (TM_CCOEFF_NORMED)。8 ビット同士と、32 ビット浮動小数点同士."""
    rng = np.random.default_rng(4)
    roi = rng.integers(0, 256, (12, 30), dtype=np.uint8)
    roi[:, 20:] = 77  # 分散 0 の窓ができるように、右側を同じ値で塗る
    t = rng.integers(0, 256, (5, 9), dtype=np.uint8)
    roi_f = rng.normal(100, 30, (14, 16)).astype(np.float32)
    t_f = rng.normal(100, 30, (6, 7)).astype(np.float32)
    _save("matching", {"roi": _img(roi), "templ": _img(t), "result": _img(cv2.matchTemplate(roi, t, cv2.TM_CCOEFF_NORMED)),
                       "roi_f": _img(roi_f), "templ_f": _img(t_f),
                       "result_f": _img(cv2.matchTemplate(roi_f, t_f, cv2.TM_CCOEFF_NORMED))})


def components():
    """連結成分 (8 近傍) の統計: 番号順に (左, 上, 幅, 高さ, 画素数)。0 番は背景."""
    rng = np.random.default_rng(5)
    m = (rng.random((15, 20)) < 0.35).astype(np.uint8)
    n, labels, stats, _ = cv2.connectedComponentsWithStats(m, connectivity=8)
    _save("components", {"mask": _img(m), "count": int(n), "stats": stats.tolist(), "labels": _img(labels)})


def translate():
    """整数の平行移動 (warpAffine)。画像の外は 0."""
    rng = np.random.default_rng(6)
    g = rng.integers(0, 256, (10, 20), dtype=np.uint8)
    out = {"src": _img(g)}
    for dx, dy in ((3, -2), (-4, 1), (0, 0)):
        m = np.float32([[1, 0, -dx], [0, 1, -dy]])  # reader.screen_rois と同じく、(dx, dy) のずれを戻す
        out[f"{dx},{dy}"] = _img(cv2.warpAffine(g, m, (g.shape[1], g.shape[0])))
    _save("translate", out)


def linalg():
    """小さな線形代数 (NumPy の結果)."""
    rng = np.random.default_rng(7)
    a3 = rng.normal(0, 1, (3, 3))
    b3 = rng.normal(0, 1, 3)
    a2 = rng.normal(0, 1, (2, 2))
    m = rng.normal(0, 1, (12, 9))
    sym = m.T @ m                      # 9x9 の対称行列 (正定値)
    w, v = np.linalg.eigh(sym)
    h = np.array([[1.2, 0.1, 30.0], [-0.05, 0.9, 12.0], [1e-4, -2e-4, 1.0]])
    _save("linalg", {
        "a3": a3.tolist(), "b3": b3.tolist(), "inv3": np.linalg.inv(a3).tolist(),
        "solve3": np.linalg.solve(a3, b3).tolist(), "det3": float(np.linalg.det(a3)),
        "a2": a2.tolist(), "sv2": np.linalg.svd(a2, compute_uv=False).tolist(),
        "sym9": sym.tolist(), "eigvals9": w.tolist(), "eigvec9_min": v[:, 0].tolist(),
        "h": h.tolist(), "cond_h": float(np.linalg.cond(h)),
        "svd_solve": {"x": __import__("charm_reader.pure_ransac", fromlist=["svd_solve"]).svd_solve(sym, b3.tolist() * 3).tolist()},
    })


def _rgb(bgr):
    """OpenCV の BGR の画像を、RGB の並びの {"width", "height", "data"} にする."""
    return {"width": int(bgr.shape[1]), "height": int(bgr.shape[0]), "data": bgr[..., ::-1].reshape(-1).tolist()}


def align_ops():
    """位置合わせで使う画像処理: 灰色への変換、写真の縮小 (INTER_AREA) と拡大 (INTER_CUBIC)、射影変換で写す
    (warpPerspective の線形補間、外は 0)、連結成分の重心."""
    rng = np.random.default_rng(8)
    bgr = rng.integers(0, 256, (47, 61, 3), dtype=np.uint8)
    out = {"src": _rgb(bgr), "gray": _img(cv2.cvtColor(bgr, cv2.COLOR_BGR2GRAY))}
    for s in (0.496, 0.3125):
        out[f"area{s}"] = _rgb(cv2.resize(bgr, None, fx=s, fy=s, interpolation=cv2.INTER_AREA))
    out["cubic1.5625"] = _rgb(cv2.resize(bgr, None, fx=1.5625, fy=1.5625, interpolation=cv2.INTER_CUBIC))
    h = np.array([[0.9, 0.12, -3.0], [-0.08, 1.05, 2.5], [0.0012, -0.0009, 1.0]])
    out["h"] = h.tolist()
    out["warp"] = _rgb(cv2.warpPerspective(bgr, h, (64, 48), flags=cv2.INTER_LINEAR))
    m = (rng.random((15, 20)) < 0.35).astype(np.uint8)
    _, _, _, cen = cv2.connectedComponentsWithStats(m, connectivity=8)
    out["cc_mask"] = _img(m)
    out["cc_centroids"] = cen[1:].tolist()
    _save("align_ops", out)


def templates():
    """見本の下ごしらえ (experimental-charm-reader の reader.templates() など) の結果.

    見本は全部で 400 枚を超えるので、全部の画素は書かず、(幅, 高さ, 画素の合計) と、いくつかの見本だけ画素を書く。
    """
    sys.path.insert(0, str(EXPERIMENTAL))
    from charm_reader import reader as R
    t = R.templates()
    out = {"scales": list(R.TMPL_SCALES), "full": {}}
    for kind in ("skill", "pts"):
        out[kind] = {str(k): [[int(a.shape[1]), int(a.shape[0]), int(a.astype(np.int64).sum())] for a in v]
                     for k, v in t[kind].items()}
    for kind, k in (("skill", 4), ("skill", 29), ("pts", 1), ("pts", 13)):
        out["full"][f"{kind}/{k}"] = [_img(a) for a in t[kind][k]]
    out["signs"] = {k: _img(v) for k, v in R.sign_templates().items()}
    out["slots"] = {k: _img(v) for k, v in R.slot_templates().items()}
    out["label"] = _img(R._label_template())
    out_dir = FIXTURES / "read"
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "templates.json").write_text(json.dumps(out) + "\n")


def _keypoints(kps, des):
    """特徴点 [x, y, 大きさ, 向き] と、記述子 (128 個の 0〜255 の整数を 16 進の文字列に)."""
    return {"kps": [[float(v) for v in k] for k in kps],
            "des": [bytes(int(v) for v in d).hex() for d in des]}


def sift():
    """型紙 (assets/reference.png) の SIFT の特徴点と記述子: NumPy 版 (pure_sift.py、Rust 版と bit-exact に合わせる相手)
    と OpenCV (Python 版の位置合わせが使うもの、どれだけ合うかを見る相手)。入力は見本だけ."""
    from charm_reader import pure_sift as P
    g = cv2.imread(str(FIXTURES.parent.parent / "assets" / "reference.png"), cv2.IMREAD_GRAYSCALE)
    out = FIXTURES / "sift"
    out.mkdir(parents=True, exist_ok=True)
    kps, des = P.SIFT(nfeatures=8000).detect_and_compute(g)
    data = _keypoints([[k.x, k.y, k.size, k.angle] for k in kps], des)
    (out / "numpy.json").write_text(json.dumps(data) + "\n")
    kps, des = cv2.SIFT_create(nfeatures=8000).detectAndCompute(g, None)
    data = _keypoints([[k.pt[0], k.pt[1], k.size, k.angle] for k in kps], des)
    (out / "opencv.json").write_text(json.dumps(data) + "\n")


def ransac():
    """RANSAC と最小二乗の射影変換 (NumPy 版の pure_ransac.py)。対応点は、人工の写真 (tests/synthetic/、
    tools/make_synthetic.py で作る) を Python 版と同じく縮小して OpenCV の SIFT で型紙と対応づけたもの."""
    from charm_reader import pure_ransac as PR
    from charm_reader.align import WORK_LONG_SIDE, Aligner
    al = Aligner()
    out = []
    for name in ("01_basic.jpg", "02_oblique.jpg", "04_far.jpg", "09_small_image.jpg"):
        img = cv2.imread(str(FIXTURES.parent / "synthetic" / name))
        s = WORK_LONG_SIDE / max(img.shape[:2])
        small = cv2.resize(img, None, fx=s, fy=s, interpolation=cv2.INTER_AREA if s < 1 else cv2.INTER_CUBIC)
        kp, des = al.detector.detectAndCompute(cv2.cvtColor(small, cv2.COLOR_BGR2GRAY), None)
        pairs = cv2.BFMatcher(cv2.NORM_L2).knnMatch(des, al.des, k=2)
        good = [q[0] for q in pairs if len(q) == 2 and q[0].distance < 0.75 * q[1].distance]
        src = np.float32([kp[m.queryIdx].pt for m in good])
        dst = np.float32([al.kp[m.trainIdx].pt for m in good])
        H, mask = PR.find_homography(src, dst, PR.RANSAC, 3.0)
        keep = mask.ravel() > 0
        H_ls = PR.find_homography(src[keep], dst[keep], 0, 3.0)[0] if keep.sum() >= 4 else None
        out.append({"name": name, "src": src.tolist(), "dst": dst.tolist(),
                    "H": None if H is None else H.tolist(), "mask": mask.ravel().tolist(),
                    "H_ls": None if H_ls is None else H_ls.tolist()})
    _save("ransac", out)


if __name__ == "__main__":
    load()
    color()
    morph()
    resize()
    matching()
    components()
    translate()
    templates()
    sys.path.insert(0, str(EXPERIMENTAL))
    linalg()
    align_ops()
    sift()
    ransac()
