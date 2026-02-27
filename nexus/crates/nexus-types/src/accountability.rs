use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareScope {
    pub collection: String,
    pub item: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Accountability {
    pub role: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    pub user: Option<String>,
    #[serde(default)]
    pub admin: bool,
    #[serde(default)]
    pub app: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share: Option<String>,
    pub ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_accountability_default() {
        let acc = Accountability::default();
        assert!(acc.role.is_none());
        assert!(acc.user.is_none());
        assert!(!acc.admin);
        assert!(!acc.app);
        assert!(acc.roles.is_empty());
        assert!(acc.share.is_none());
        assert!(acc.ip.is_none());
        assert!(acc.session.is_none());
    }

    #[test]
    fn test_accountability_serialization() {
        let acc = Accountability {
            role: Some("admin-role-id".to_string()),
            roles: vec!["admin-role-id".to_string()],
            user: Some("user-123".to_string()),
            admin: true,
            app: true,
            ip: Some("127.0.0.1".to_string()),
            ..Default::default()
        };
        let val = serde_json::to_value(&acc).unwrap();
        assert_eq!(val["role"], json!("admin-role-id"));
        assert_eq!(val["user"], json!("user-123"));
        assert_eq!(val["admin"], json!(true));
        assert_eq!(val["app"], json!(true));
        assert_eq!(val["ip"], json!("127.0.0.1"));
        assert_eq!(val["roles"], json!(["admin-role-id"]));
    }

    #[test]
    fn test_accountability_deserialization() {
        let json_str = r#"{
            "role": "editor",
            "roles": ["editor", "viewer"],
            "user": "u-1",
            "admin": false,
            "app": true,
            "ip": "192.168.1.1"
        }"#;
        let acc: Accountability = serde_json::from_str(json_str).unwrap();
        assert_eq!(acc.role.unwrap(), "editor");
        assert_eq!(acc.roles, vec!["editor", "viewer"]);
        assert_eq!(acc.user.unwrap(), "u-1");
        assert!(!acc.admin);
        assert!(acc.app);
        assert_eq!(acc.ip.unwrap(), "192.168.1.1");
    }

    #[test]
    fn test_accountability_deserialization_minimal() {
        let json_str = r#"{}"#;
        let acc: Accountability = serde_json::from_str(json_str).unwrap();
        assert!(acc.role.is_none());
        assert!(acc.roles.is_empty());
        assert!(!acc.admin);
        assert!(!acc.app);
    }

    #[test]
    fn test_share_scope() {
        let scope = ShareScope {
            collection: "articles".to_string(),
            item: "item-uuid".to_string(),
        };
        let val = serde_json::to_value(&scope).unwrap();
        assert_eq!(val["collection"], json!("articles"));
        assert_eq!(val["item"], json!("item-uuid"));
    }
}
