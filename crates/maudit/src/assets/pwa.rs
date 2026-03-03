use rolldown::plugin::{HookUsage, Plugin};

pub const PWA_REGISTER_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/js/pwa/register.ts");

/// Rolldown plugin to handle the maudit:pwa specifier.
/// Resolves `maudit:pwa` to the actual file path of pwa/register.ts in the Maudit crate.
#[derive(Debug)]
pub struct PwaPlugin;

impl Plugin for PwaPlugin {
    fn name(&self) -> std::borrow::Cow<'static, str> {
        "builtin:pwa".into()
    }

    fn register_hook_usage(&self) -> HookUsage {
        HookUsage::ResolveId
    }

    async fn resolve_id(
        &self,
        _ctx: &rolldown::plugin::PluginContext,
        args: &rolldown::plugin::HookResolveIdArgs<'_>,
    ) -> rolldown::plugin::HookResolveIdReturn {
        if args.specifier == "maudit:pwa" {
            return Ok(Some(rolldown::plugin::HookResolveIdOutput {
                id: PWA_REGISTER_PATH.into(),
                ..Default::default()
            }));
        }
        Ok(None)
    }
}
