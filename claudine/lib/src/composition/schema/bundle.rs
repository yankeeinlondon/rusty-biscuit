//! Claudine's registration, compiled into the binary, as a registered host.
//!
//! The registration inventory ([`crate::composition::inventory`], the files
//! of `claudine/registration/`) is the bundle Claudine publishes for
//! Darkmatter. In process, Claudine judges its documents with the same
//! bundle without reading the published copy: [`claudine_profile`] verifies
//! the compiled manifest and members from memory
//! ([`VerifiedSnapshot::from_embedded`]) and loads them as a registered host
//! profile ([`registration::load_embedded`]), the path an installed
//! registration takes. Its triggers therefore layer as one checked host
//! layer, root unions included (`inline-composition.yaml`'s `prompt` /
//! `sections` arms), below every ordinary schema-root trigger, which keeps
//! its higher precedence. Its triggers' origins are
//! `embedded:claudine/schemas/<file>`; no host file is named.
//!
//! The trigger payloads are served prepared from Claudine's shipped artifact
//! ([`super::shipped`]) while it is accepted, and prepared from the
//! compiled text otherwise. See `claudine/docs/topics/schemas.md`.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use darkmatter::config::registration::bundle::{SnapshotRefusal, VerifiedSnapshot};
use darkmatter::config::registration::{
    LoadedRegistrations, RegisteredHostProfile, RegisteredHosts, load_embedded, md_owned_names,
};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use simplified_schema::SchemaOriginKind;
use simplified_schema::triggers::TriggerOrigin;
use simplified_schema::triggers::assemble::ResolvedPayload;

use crate::composition::inventory::{self, InventoryMember, MemberKind, members};
use crate::composition::registration::{MANIFEST_FILE, RegistrationInventory};

/// The bundle's name in origins, traces, and errors
/// (`embedded:claudine/schemas/…`).
pub const CLAUDINE_SCHEMA_BUNDLE_NAME: &str = inventory::BUNDLE_NAME;

const fn count_triggers() -> usize {
    let all = members();
    let (mut index, mut found) = (0, 0);
    while index < all.len() {
        if matches!(all[index].kind, MemberKind::Trigger) {
            found += 1;
        }
        index += 1;
    }
    found
}

const TRIGGER_COUNT: usize = count_triggers();

const fn trigger_files() -> [&'static str; TRIGGER_COUNT] {
    let all = members();
    let mut files = [""; TRIGGER_COUNT];
    let (mut index, mut next) = (0, 0);
    while index < all.len() {
        if matches!(all[index].kind, MemberKind::Trigger) {
            files[next] = all[index].path;
            next += 1;
        }
        index += 1;
    }
    files
}

const TRIGGER_FILES: [&str; TRIGGER_COUNT] = trigger_files();

/// The inventory paths of the trigger members (`composition.yaml`), in
/// inventory order.
pub const CLAUDINE_TRIGGER_FILES: &[&str] = &TRIGGER_FILES;

/// The registered profile of `inventory`, verified and prepared from its
/// bytes alone; payloads `prepared` serves (by trigger bundle path) are not
/// resolved again.
///
/// ## Errors
///
/// Every refusal registration reports for the bundle.
pub fn profile_of_with(
    inventory: &RegistrationInventory,
    prepared: &dyn Fn(&str) -> Option<ResolvedPayload>,
) -> Result<RegisteredHostProfile, Vec<SnapshotRefusal>> {
    let snapshot = snapshot_of(inventory).map_err(|refusal| vec![*refusal])?;
    load_embedded(snapshot, md_owned_names(), prepared)
}

/// [`profile_of_with`] preparing every payload from text: for an edited
/// inventory, such as a test's.
///
/// ## Errors
///
/// As [`profile_of_with`].
pub fn profile_of(inventory: &RegistrationInventory) -> Result<RegisteredHostProfile, Vec<SnapshotRefusal>> {
    profile_of_with(inventory, &|_| None)
}

/// The verified embedded snapshot of `inventory`.
///
/// ## Errors
///
/// The snapshot's refusal.
pub fn snapshot_of(inventory: &RegistrationInventory) -> Result<VerifiedSnapshot, Box<SnapshotRefusal>> {
    let members: BTreeMap<String, Arc<[u8]>> =
        inventory.members().map(|file| (file.path().to_string(), Arc::from(file.bytes()))).collect();
    VerifiedSnapshot::from_embedded(CLAUDINE_SCHEMA_BUNDLE_NAME, MANIFEST_FILE, inventory.manifest().bytes(), &members)
}

/// `profile` as the only registered host of a request.
#[must_use]
pub fn hosts_of(profile: RegisteredHostProfile) -> Arc<RegisteredHosts> {
    let loaded = LoadedRegistrations { profiles: vec![Arc::new(profile)], diagnostics: Vec::new(), watches: Vec::new() };
    Arc::new(RegisteredHosts::new(Arc::new(loaded)))
}

/// The registered hosts of an in-process launch: Claudine's compiled
/// registration alone, built once per process.
///
/// ## Panics
///
/// When the compiled registration does not load: a build defect the
/// registration tests catch.
pub fn claudine_hosts() -> &'static Arc<RegisteredHosts> {
    static HOSTS: OnceLock<Arc<RegisteredHosts>> = OnceLock::new();
    HOSTS.get_or_init(|| {
        let profile = profile_of_with(RegistrationInventory::claudine(), &super::shipped::registered_payload)
            .unwrap_or_else(|refusals| panic!("Claudine's compiled registration must load: {refusals:#?}"));
        hosts_of(profile)
    })
}

/// Claudine's compiled registration as a registered host profile
/// ([`claudine_hosts`]'s one profile).
///
/// ## Panics
///
/// As [`claudine_hosts`].
pub fn claudine_profile() -> &'static Arc<RegisteredHostProfile> {
    static PROFILE: OnceLock<Arc<RegisteredHostProfile>> = OnceLock::new();
    PROFILE.get_or_init(|| {
        let hosts = claudine_hosts();
        match hosts.claims() {
            [darkmatter::config::registration::HostClaim::Agreed { profile, .. }] => Arc::clone(profile),
            other => panic!("Claudine's hosts hold one agreed claim, not {other:?}"),
        }
    })
}

/// `schemas` judging with Claudine's compiled registration as the
/// registered host layer, below the schema roots' triggers.
#[must_use]
pub fn with_claudine_registration(schemas: DarkmatterSchemas) -> DarkmatterSchemas {
    schemas.with_registered_hosts(Arc::clone(claudine_hosts()))
}

/// The inventory member of the trigger bundle path `path`
/// (`schemas/composition.yaml`).
#[must_use]
pub fn trigger_member(path: &str) -> Option<&'static InventoryMember> {
    let path = path.strip_prefix(inventory::MEMBER_FOLDER)?.strip_prefix('/')?;
    inventory::member(path).filter(|member| member.kind == MemberKind::Trigger)
}

/// Whether `property`'s winning declaration in `effective` is a Claudine
/// trigger declaration: the winner is a trigger of the registered Claudine
/// host layer, compiled in or installed.
///
/// The winner is Darkmatter's per-property origin: an authored `$schema`, or
/// a schema-root trigger, that redeclares the property takes it over, so
/// that declaration (and its cleanup policy) applies instead. A disk copy of
/// a Claudine registration file in a schema root is such an ordinary
/// trigger, never Claudine's.
#[must_use]
pub fn is_claudine_owned(effective: &EffectiveSchema, property: &str) -> bool {
    let Some(origin) = effective.origins.get(property) else {
        return false;
    };
    if origin.kind != SchemaOriginKind::Trigger {
        return false;
    }
    let Some(source) = origin.trigger_source() else {
        return false;
    };
    let layers = effective.layers();
    let Some(registered) = layers.registered.as_ref().filter(|registered| registered.host == crate::composition::registration::HOST_ID)
    else {
        return false;
    };
    // The registered host's triggers layer first, in its own order.
    layers.triggers.iter().take(registered.triggers.len()).any(|layered| *layered == source)
}
