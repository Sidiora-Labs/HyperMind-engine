use super::believe::belief_type_name;
use crate::Envelope;
use hm_schema::events::BeliefType;
use hm_serve::actor::ActorEngine;
use hm_serve::config::CapabilityToken;
use serde_json::json;

pub(crate) const ADMIN_TOKEN_REQUIRED_ACTIONS: [(&str, &[&str]); 1] =
    [("forget", &["crypto_shred"])];

pub(crate) fn run(actor: &ActorEngine, admin_token: Option<&CapabilityToken>) -> Envelope {
    let verbs = hm_serve::rest::VERBS
        .iter()
        .map(|verb| {
            json!({
                "verb": verb,
                "mutating": hm_serve::uds::mutation_verb(verb),
                "admin_listener_required": false,
                "admin_token_required_actions": admin_token_required_actions(verb),
            })
        })
        .collect::<Vec<_>>();
    let protected = BeliefType::ENUM_VALUES
        .iter()
        .filter(|belief_type| is_protected(**belief_type))
        .map(|belief_type| belief_type_name(*belief_type))
        .collect::<Vec<_>>();
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "surface": "access",
        "source": "server",
        "actor": actor.actor().get(),
        "isolation": "actor_scoped",
        "admin_token_configured": admin_token.is_some(),
        "admin_operations": hm_serve::auth::ADMIN_OPERATIONS,
        "verbs": verbs,
        "protected_belief_types": protected,
    }));
    envelope
}

fn admin_token_required_actions(verb: &str) -> Vec<&'static str> {
    ADMIN_TOKEN_REQUIRED_ACTIONS
        .iter()
        .find(|(name, _)| *name == verb)
        .map(|(_, actions)| actions.to_vec())
        .unwrap_or_default()
}

const fn is_protected(belief_type: BeliefType) -> bool {
    matches!(
        belief_type,
        BeliefType::Identity | BeliefType::Constraint | BeliefType::Preference
    )
}
