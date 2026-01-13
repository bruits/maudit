use std::path::PathBuf;

use rolldown::plugin::{HookUsage, Plugin};

/// Rolldown plugin to resolve prefetch modules to their actual file paths.
#[derive(Debug)]
pub struct PrefetchPlugin;

impl PrefetchPlugin {
    /// Get the base directory where the prefetch files are located.
    fn get_base_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("js")
    }

    /// Resolve a maudit:prefetch specifier to its actual file path.
    fn resolve_prefetch_path(specifier: &str) -> Option<PathBuf> {
        let base_dir = Self::get_base_dir();

        match specifier {
            "maudit:prefetch" => Some(base_dir.join("prefetch.ts")),
            "maudit:prefetch:hover" => Some(base_dir.join("prefetch").join("hover.ts")),
            _ => None,
        }
    }
}

impl Plugin for PrefetchPlugin {
    fn name(&self) -> std::borrow::Cow<'static, str> {
        "builtin:prefetch".into()
    }

    fn register_hook_usage(&self) -> HookUsage {
        HookUsage::ResolveId
    }

    async fn resolve_id(
        &self,
        _ctx: &rolldown::plugin::PluginContext,
        args: &rolldown::plugin::HookResolveIdArgs<'_>,
    ) -> rolldown::plugin::HookResolveIdReturn {
        if let Some(path) = Self::resolve_prefetch_path(args.specifier) {
            return Ok(Some(rolldown::plugin::HookResolveIdOutput {
                id: path.to_string_lossy().to_string().into(),
                ..Default::default()
            }));
        }
        Ok(None)
    }
}
