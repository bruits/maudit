use rolldown::plugin::{HookUsage, Plugin};

pub const PREFETCH_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/js/prefetch.ts");
pub const PREFETCH_HOVER_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/js/prefetch/hover.ts");
pub const PREFETCH_TAP_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/js/prefetch/tap.ts");
pub const PREFETCH_VIEWPORT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/js/prefetch/viewport.ts");

// Built paths, we don't use any of those ourselves but they can be useful if someone wants to have a bundler-less Maudit
pub const PREFETCH_BUILT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/js/dist/prefetch.js");
pub const PREFETCH_HOVER_BUILT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/js/dist/prefetch/hover.js");
pub const PREFETCH_TAP_BUILT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/js/dist/prefetch/tap.js");
pub const PREFETCH_VIEWPORT_BUILT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/js/dist/prefetch/viewport.js");

/// Rolldown plugin to handle the maudit:prefetch specifier.
/// Importing the actual prefetch.ts file from Maudit's crate is very cumbersome in JS, and TypeScript anyway won't enjoy finding the types there
/// As such, this plugin resolves the maudit:prefetch specifier to the actual file path of prefetch.ts in the Maudit crate for the user.
#[derive(Debug)]
pub struct PrefetchPlugin;

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
        if args.specifier == "maudit:prefetch" {
            return Ok(Some(rolldown::plugin::HookResolveIdOutput {
                id: PREFETCH_PATH.into(),
                ..Default::default()
            }));
        }
        Ok(None)
    }
}
