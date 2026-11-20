//! Text embeddings. [`HashEmbedder`] is deterministic and offline (tests, fallback);
//! a fastembed-backed embedder can be enabled with the `fastembed` feature.

use hangouts_core::Event;

use crate::error::RecommendError;

/// Turns texts into vectors of [`Embedder::dim`] floats, L2-normalized.
pub trait Embedder: Send + Sync {
    /// Vector length.
    fn dim(&self) -> usize;
    /// One vector per input text, in order.
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, RecommendError>;
}

/// Feature-hashing bag-of-words embedder: no model download, same output everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HashEmbedder {
    dim: usize,
}

impl HashEmbedder {
    /// Default vector length.
    pub const DEFAULT_DIM: usize = 256;

    /// An embedder producing `dim`-length vectors (`dim` ≥ 1).
    pub fn new(dim: usize) -> Self {
        Self { dim: dim.max(1) }
    }
}

impl Default for HashEmbedder {
    fn default() -> Self {
        Self::new(Self::DEFAULT_DIM)
    }
}

/// FNV-1a, stable across platforms and Rust versions (unlike `DefaultHasher`).
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

impl Embedder for HashEmbedder {
    fn dim(&self) -> usize {
        self.dim
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, RecommendError> {
        Ok(texts
            .iter()
            .map(|text| {
                let mut v = vec![0.0_f32; self.dim];
                let lower = text.to_lowercase();
                for token in lower
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|t| !t.is_empty())
                {
                    let h = fnv1a(token.as_bytes());
                    let idx = (h % self.dim as u64) as usize;
                    v[idx] += if h >> 63 == 0 { 1.0 } else { -1.0 };
                }
                let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm > 0.0 {
                    for x in &mut v {
                        *x /= norm;
                    }
                }
                v
            })
            .collect())
    }
}

/// The embedder the app should use: fastembed when the `fastembed` feature is enabled and
/// its model loads, otherwise [`HashEmbedder`]. Never fails.
pub fn default_embedder() -> Box<dyn Embedder> {
    #[cfg(feature = "fastembed")]
    {
        // A failed model download/load falls through to the hash embedder.
        if let Ok(e) = fast::FastEmbedder::new() {
            return Box::new(e);
        }
    }
    Box::new(HashEmbedder::default())
}

/// Embeds `texts` and returns `None` on failure, for callers that treat embeddings as optional.
pub(crate) fn try_embed(embedder: &dyn Embedder, texts: &[String]) -> Option<Vec<Vec<f32>>> {
    embedder
        .embed(texts)
        .ok()
        .filter(|v| v.len() == texts.len())
}

/// Cosine similarity; 0 when either vector is zero or the lengths differ.
pub fn cosine(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0_f64, 0.0_f64, 0.0_f64);
    for (x, y) in a.iter().zip(b) {
        let (x, y) = (f64::from(*x), f64::from(*y));
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

/// Mean of `vectors`, L2-normalized; `None` if empty or all zero.
pub fn mean_normalized(vectors: &[&[f32]]) -> Option<Vec<f32>> {
    let dim = vectors.first()?.len();
    let mut out = vec![0.0_f32; dim];
    for v in vectors.iter().filter(|v| v.len() == dim) {
        for (o, x) in out.iter_mut().zip(v.iter()) {
            *o += x;
        }
    }
    let norm = out.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return None;
    }
    for x in &mut out {
        *x /= norm;
    }
    Some(out)
}

#[cfg(feature = "fastembed")]
mod fast {
    use std::sync::Mutex;

    use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};

    use super::Embedder;
    use crate::error::RecommendError;

    /// all-MiniLM-L6-v2 sentence embeddings via fastembed (384 dims). Downloads the model on
    /// first use into fastembed's cache directory.
    pub struct FastEmbedder {
        model: Mutex<TextEmbedding>,
    }

    impl FastEmbedder {
        /// Output length of all-MiniLM-L6-v2.
        pub const DIM: usize = 384;

        /// Loads (and if needed downloads) the model.
        pub fn new() -> Result<Self, RecommendError> {
            let model = TextEmbedding::try_new(TextInitOptions::new(EmbeddingModel::AllMiniLML6V2))
                .map_err(|e| RecommendError::Embedding(e.to_string()))?;
            Ok(Self {
                model: Mutex::new(model),
            })
        }
    }

    impl Embedder for FastEmbedder {
        fn dim(&self) -> usize {
            Self::DIM
        }

        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, RecommendError> {
            let mut model = self
                .model
                .lock()
                .map_err(|_| RecommendError::Embedding("embedder lock poisoned".to_owned()))?;
            let mut out = model
                .embed(texts, None)
                .map_err(|e| RecommendError::Embedding(e.to_string()))?;
            for v in &mut out {
                let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm > 0.0 {
                    for x in v.iter_mut() {
                        *x /= norm;
                    }
                }
            }
            Ok(out)
        }
    }
}

#[cfg(feature = "fastembed")]
pub use fast::FastEmbedder;

/// The text embedded for an event: title, description, and tags.
pub fn event_text(event: &Event) -> String {
    format!(
        "{}\n{}\n{}",
        event.title,
        event.description,
        event.tags.join(" ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_embedder_is_deterministic_and_normalized() {
        let e = HashEmbedder::default();
        let texts = vec![
            "Smash Bros night".to_owned(),
            "smash bros NIGHT!".to_owned(),
            String::new(),
        ];
        let v = e.embed(&texts).unwrap();
        assert_eq!(v[0], v[1]);
        assert_eq!(v[0].len(), HashEmbedder::DEFAULT_DIM);
        let norm: f32 = v[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
        assert!(v[2].iter().all(|x| *x == 0.0));
    }

    #[test]
    fn cosine_and_mean() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-9);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-9);
        assert!(cosine(&[0.0, 0.0], &[1.0, 0.0]).abs() < 1e-9);
        assert!(cosine(&[1.0], &[1.0, 0.0]).abs() < 1e-9);
        let m = mean_normalized(&[&[1.0, 0.0], &[0.0, 1.0]]).unwrap();
        assert!(
            (m[0] - m[1]).abs() < 1e-6 && (m[0] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5
        );
        assert_eq!(mean_normalized(&[]), None);
    }

    #[test]
    fn related_texts_are_closer_than_unrelated() {
        let e = HashEmbedder::default();
        let v = e
            .embed(&[
                "board games tabletop".to_owned(),
                "Tabletop Tuesday\nCatan and Codenames\nboard-games tabletop".to_owned(),
                "Pickup Basketball\nRun fives\nbasketball fitness".to_owned(),
            ])
            .unwrap();
        assert!(cosine(&v[0], &v[1]) > cosine(&v[0], &v[2]) + 0.3);
    }
}
