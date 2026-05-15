use std::path::{Path, PathBuf};

use crate::BuildOutput;

/// A URL pointing to an asset Maudit manages.
///
/// An [`AssetUrl`] has two forms:
///
/// - The *rendered* form (via [`AssetUrl::as_rendered`]) is the URL string Maudit
///   embeds in HTML during page render. In library-mode builds this is also the
///   final URL — it matches where [`Asset::build_path`] points.
/// - The *resolved* form (via [`AssetUrl::resolve`]) is the URL after coronate's
///   post-bundle pass has rewritten it to the content-hashed filename Rolldown /
///   lightningcss produced. Library-mode pipelines that don't run a substitution
///   pass see the same value as the rendered form.
///
/// The newtype exists to keep the distinction visible: there's no `Display` /
/// `AsRef<str>` impl, so `println!("{}", script.url())` is a compile error. Pick
/// `as_rendered()` when you're emitting HTML, `resolve(&BuildOutput)` when you
/// want the on-disk URL after a build has finished.
///
/// [`Asset::build_path`]: super::Asset::build_path
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetUrl {
    pub(crate) rendered: String,
}

impl AssetUrl {
    pub(crate) fn new(rendered: String) -> Self {
        Self { rendered }
    }

    /// The URL Maudit embeds in HTML during render. Coronate's substitution pass
    /// may rewrite the resulting HTML to point at a content-hashed URL — this
    /// method returns the intermediate value, not the post-build one.
    pub fn as_rendered(&self) -> &str {
        &self.rendered
    }

    /// The post-build URL, consulting `output`'s substitution map. Falls back to
    /// [`as_rendered`](Self::as_rendered) when no substitution was applied (the
    /// usual case for library-mode pipelines or for assets coronate didn't bundle).
    pub fn resolve<'a>(&'a self, output: &'a BuildOutput) -> &'a str {
        output
            .resolve_asset_url(&self.rendered)
            .unwrap_or(&self.rendered)
    }
}

#[cfg(feature = "maud")]
impl maud::Render for AssetUrl {
    fn render_to(&self, buffer: &mut String) {
        // URLs we generate are restricted to sanitized filenames and don't contain
        // HTML metacharacters, so this is safe to emit unescaped. We still go via
        // the default escape path for paranoia in case someone customizes
        // `assets_dir` to something exotic.
        self.rendered.render_to(buffer);
    }
}

/// A filesystem path for an asset Maudit manages. The on-disk parallel of
/// [`AssetUrl`] — same rendered/resolved split.
///
/// [`Asset::build_path`] returns one of these so that callers consciously choose
/// between the path Maudit intended to write (e.g. for library-mode `fs::copy`
/// pipelines) and the path the file actually lives at after coronate's bundling
/// step (which may produce a content-hashed filename).
///
/// [`Asset::build_path`]: super::Asset::build_path
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetPath {
    pub(crate) rendered: PathBuf,
}

impl AssetPath {
    pub(crate) fn new(rendered: PathBuf) -> Self {
        Self { rendered }
    }

    /// The on-disk path Maudit intended for this asset. In a library-mode build
    /// pipeline (no coronate substitution) this is where the file actually lives;
    /// `fs::copy(asset.path(), asset.build_path().as_rendered())` is the canonical
    /// "copy the source to its destination" pattern.
    pub fn as_rendered(&self) -> &Path {
        &self.rendered
    }

    /// The actual on-disk path after a build, consulting `output`'s substitution
    /// map. Falls back to [`as_rendered`](Self::as_rendered) when no substitution
    /// was applied.
    pub fn resolve<'a>(&'a self, output: &'a BuildOutput) -> &'a Path {
        output
            .resolve_asset_path(&self.rendered)
            .unwrap_or(&self.rendered)
    }
}
