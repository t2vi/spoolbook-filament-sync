use crate::cloak_browser_client::CloakBrowserClient;
use crate::filament_sync_entry::FilamentSyncEntry;

// One implementation per brand — replaces the old XStoreClient + XStoreParser + SyncXAsync
// triad. `cloak` is None for every source except Hatchbox, which sits behind an anti-bot wall.
#[async_trait::async_trait]
pub trait FilamentSource {
    fn name(&self) -> &'static str;

    async fn fetch(
        &self,
        cloak: Option<&CloakBrowserClient>,
    ) -> Result<Vec<FilamentSyncEntry>, String>;
}
