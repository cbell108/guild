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
    // Lane C (C3): when built with `--features fastembed`, try the fastembed model here and
    // fall back to the hash embedder on any error.
    Box::new(HashEmbedder::default())
}

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
}
