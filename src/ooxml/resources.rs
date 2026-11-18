//! Per-compilation resource registration. Hashes index candidates; bytes prove equality.
use crate::opc::Package;
use crate::{AssetId, ResolvedImage, Result};
use std::{
    collections::{BTreeMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};
#[derive(Default)]
pub(super) struct MediaRegistry {
    by_hash: BTreeMap<u64, Vec<(AssetId, String)>>,
    next: AssetId,
}
fn fingerprint(data: &[u8]) -> u64 {
    let mut hash = DefaultHasher::new();
    data.hash(&mut hash);
    hash.finish()
}
impl MediaRegistry {
    pub fn from_package(package: &Package) -> Result<Self> {
        let mut registry = Self::default();
        for (path, bytes) in &package.parts {
            if path.rsplit('.').next().is_some_and(|ext| {
                ["png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif"]
                    .contains(&ext.to_ascii_lowercase().as_str())
            }) {
                registry.next = registry.next.next()?;
                registry
                    .by_hash
                    .entry(fingerprint(bytes))
                    .or_default()
                    .push((registry.next, path.clone()));
            }
        }
        Ok(registry)
    }
    pub fn register(&mut self, package: &mut Package, image: &ResolvedImage) -> Result<String> {
        let key = fingerprint(&image.data);
        if let Some(candidates) = self.by_hash.get(&key)
            && let Some((_, path)) = candidates.iter().find(|(_, path)| {
                package
                    .parts
                    .get(path)
                    .is_some_and(|bytes| *bytes == image.data)
            })
        {
            return Ok(path.clone());
        }
        let path = package.unique("word/media/image", &image.extension);
        package.parts.insert(path.clone(), image.data.clone());
        package.content_type(&path, &image.content_type)?;
        self.next = self.next.next()?;
        self.by_hash
            .entry(key)
            .or_default()
            .push((self.next, path.clone()));
        Ok(path)
    }
}
