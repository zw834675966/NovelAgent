//! Topcoat web UI: chat page + LLM / character procedure endpoints.

mod character;
mod chat;

use anyhow::{Context, Result};
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::{Router, RouterBuilderDiscoverExt};

/// Build the Topcoat router with discovered pages/procedures and assets.
///
/// The chat page embeds Topcoat runtime (and optional dev) scripts via
/// `asset!`, so a bundle next to the binary is required. Produce it with:
///
/// ```text
/// topcoat asset bundle
/// # or day-to-day: topcoat dev
/// ```
///
/// # Errors
///
/// Returns an error when the asset bundle beside the executable is missing
/// or unreadable.
pub fn router() -> Result<Router> {
    let bundle = AssetBundle::load().context(
        "topcoat asset bundle missing next to the binary; run `topcoat asset bundle` or `topcoat dev` first",
    )?;
    Ok(Router::builder().discover().assets(bundle).build())
}
