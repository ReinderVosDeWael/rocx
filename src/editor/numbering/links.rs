//! Validate reciprocal numbering-style ownership without rewriting either link.
use super::*;

pub(super) struct StyleLinks {
    by_instance: BTreeMap<u32, Vec<String>>,
    by_style: BTreeMap<String, u32>,
}

impl StyleLinks {
    pub(super) fn load(pkg: &Package, main: &str) -> Result<Self> {
        let styles = super::super::resources::style_part(pkg, main)?;
        Self::from_styles(styles.as_ref().map(|(_, root)| root))
    }

    pub(super) fn from_styles(styles: Option<&Node>) -> Result<Self> {
        let by_instance = styles.map(view::style_ids).transpose()?.unwrap_or_default();
        let by_style = by_instance
            .iter()
            .flat_map(|(n, names)| names.iter().map(|s| (s.clone(), *n)))
            .collect();
        Ok(Self {
            by_instance,
            by_style,
        })
    }

    pub(super) fn owns(&self, num: u32) -> bool {
        self.by_instance.contains_key(&num)
    }

    /// Return the local abstract providing levels after checking both ends.
    pub(super) fn provider(&self, root: &Node, nums: &BTreeMap<u32, u32>, aid: u32) -> Result<u32> {
        let abstract_node = |aid| {
            root.elements()
                .find(|n| n.name == "w:abstractNum" && id(n, "w:abstractNumId").ok() == Some(aid))
                .ok_or_else(|| unsupported("dangling abstract numbering link"))
        };
        let source = abstract_node(aid)?;
        let link = view::optional_value(source, "w:styleLink")?;
        let inherits = view::optional_value(source, "w:numStyleLink")?;
        if let Some(name) = inherits {
            if link.is_some()
                || source.child("w:lvl").is_some()
                || nums
                    .iter()
                    .any(|(num, target)| *target == aid && self.owns(*num))
            {
                return Err(unsupported("ambiguous inherited numbering abstract"));
            }
            let owner = self
                .by_style
                .get(&name)
                .and_then(|n| nums.get(n))
                .copied()
                .ok_or_else(|| unsupported("dangling numbering style link"))?;
            let local = abstract_node(owner)?;
            if owner == aid
                || view::optional_value(local, "w:numStyleLink")?.is_some()
                || view::optional_value(local, "w:styleLink")?.as_deref() != Some(&name)
            {
                return Err(unsupported("nonreciprocal or cyclic numbering style link"));
            }
            self.local(root, nums, owner, local, Some(&name))?;
            return Ok(owner);
        }
        self.local(root, nums, aid, source, link.as_deref())?;
        Ok(aid)
    }

    fn local(
        &self,
        root: &Node,
        nums: &BTreeMap<u32, u32>,
        aid: u32,
        node: &Node,
        link: Option<&str>,
    ) -> Result<()> {
        let mut actual = nums
            .iter()
            .filter(|(_, target)| **target == aid)
            .flat_map(|(num, _)| self.by_instance.get(num).into_iter().flatten());
        match (link, actual.next()) {
            (None, None) => return Ok(()),
            (Some(expected), Some(name)) if name == expected && actual.next().is_none() => (),
            _ => {
                return Err(unsupported(
                    "dangling or nonreciprocal numbering style ownership",
                ));
            }
        }
        if node.child("w:lvl").is_none() {
            return Err(unsupported("numbering style owner has no local levels"));
        }
        // Multiple physical owners for the same style are ambiguous even if
        // all existing paragraph references happen to use one instance.
        if root
            .elements()
            .filter(|n| n.name == "w:abstractNum")
            .filter(|n| {
                n.elements()
                    .any(|l| l.name == "w:styleLink" && l.get("w:val") == link)
            })
            .count()
            != 1
        {
            return Err(unsupported("multiple abstracts claim one numbering style"));
        }
        for alias in root.elements().filter(|n| {
            n.name == "w:abstractNum"
                && n.elements()
                    .any(|l| l.name == "w:numStyleLink" && l.get("w:val") == link)
        }) {
            let alias_id = id(alias, "w:abstractNumId")?;
            if view::optional_value(alias, "w:numStyleLink")?.as_deref() != link
                || view::optional_value(alias, "w:styleLink")?.is_some()
                || alias.child("w:lvl").is_some()
                || nums
                    .iter()
                    .any(|(num, target)| *target == alias_id && self.owns(*num))
            {
                return Err(unsupported("ambiguous alias of the numbering style owner"));
            }
        }
        Ok(())
    }
}

pub(in crate::editor) fn validate_start_links(
    pkg: &Package,
    main: &str,
    root: &Node,
    ids: impl IntoIterator<Item = u32>,
) -> Result<()> {
    let links = StyleLinks::load(pkg, main)?;
    let nums = instances(root)?
        .into_iter()
        .map(|n| (n.num_id, n.abstract_id))
        .collect();
    for id in ids {
        if root
            .elements()
            .any(|n| n.name == "w:abstractNum" && super::id(n, "w:abstractNumId").ok() == Some(id))
            && links.provider(root, &nums, id)? != id
        {
            return Err(unsupported(
                "inherited abstract has no direct editable level",
            ));
        }
    }
    Ok(())
}
