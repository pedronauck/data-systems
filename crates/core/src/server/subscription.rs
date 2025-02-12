use fuel_streams_subject::subject::SubjectPayload;
use fuel_web_utils::server::middlewares::api_key::ApiKey;
use serde::{Deserialize, Serialize};

use super::DeliverPolicy;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub id: String,
    pub deliver_policy: DeliverPolicy,
    pub payload: SubjectPayload,
}

impl Subscription {
    pub fn new(
        api_key: &ApiKey,
        deliver_policy: &DeliverPolicy,
        payload: &SubjectPayload,
    ) -> Self {
        Self {
            id: Self::create_subscription_id(api_key, payload),
            deliver_policy: deliver_policy.to_owned(),
            payload: payload.to_owned(),
        }
    }

    fn create_subscription_id(
        api_key: &ApiKey,
        payload: &SubjectPayload,
    ) -> String {
        format!("{}-{}-{}", api_key.id(), api_key.user(), payload)
    }
}

impl std::fmt::Display for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_subscription_serialization() {
        let api_key =
            ApiKey::new(2.into(), "test_user".into(), "test_api_key".into());
        let deliver_policy = DeliverPolicy::FromBlock {
            block_height: 123u64.into(),
        };
        let payload = SubjectPayload {
            subject: "test_subject".into(),
            params: json!({}),
        };

        let subscription =
            Subscription::new(&api_key, &deliver_policy, &payload);

        // Test serialization
        let json = serde_json::to_string(&subscription).unwrap();
        let expected = r#"{"id":"2-test_user-test_subject:{}","deliverPolicy":{"fromBlock":{"blockHeight":123}},"payload":{"subject":"test_subject","params":{}}}"#;
        assert_eq!(json, expected);

        // Test deserialization
        let deserialized: Subscription = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, subscription);
    }

    #[test]
    fn test_subscription_with_new_policy() {
        let api_key =
            ApiKey::new(2.into(), "test_user".into(), "test_api_key".into());
        let deliver_policy = DeliverPolicy::New;
        let payload = SubjectPayload {
            subject: "test_subject".into(),
            params: json!({}),
        };
        let subscription =
            Subscription::new(&api_key, &deliver_policy, &payload);

        // Test serialization
        let json = serde_json::to_string(&subscription).unwrap();
        let expected = r#"{"id":"2-test_user-test_subject:{}","deliverPolicy":"new","payload":{"subject":"test_subject","params":{}}}"#;
        assert_eq!(json, expected);

        // Test deserialization
        let deserialized: Subscription = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, subscription);
    }
}
