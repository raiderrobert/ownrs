use serde::Deserialize;

/// Extract the owner from catalog-info.yaml content.
///
/// Reads `spec.owner` and strips the `group:` prefix if present.
pub fn extract_owner(content: &str) -> Option<String> {
    // Handle multi-document YAML — take the first document
    let doc: CatalogInfo = serde_yaml::from_str(content).ok()?;
    let owner = doc.spec?.owner?;
    Some(strip_prefix(&owner))
}

fn strip_prefix(owner: &str) -> String {
    owner
        .strip_prefix("group:")
        .or_else(|| owner.strip_prefix("user:"))
        .unwrap_or(owner)
        .to_string()
}

#[derive(Deserialize)]
struct CatalogInfo {
    spec: Option<Spec>,
}

#[derive(Deserialize)]
struct Spec {
    owner: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_group_owner() {
        let yaml = "apiVersion: backstage.io/v1alpha1\nkind: Component\nspec:\n  owner: group:platform-team\n";
        assert_eq!(extract_owner(yaml), Some("platform-team".to_string()));
    }

    #[test]
    fn no_group_prefix() {
        let yaml =
            "apiVersion: backstage.io/v1alpha1\nkind: Component\nspec:\n  owner: platform-team\n";
        assert_eq!(extract_owner(yaml), Some("platform-team".to_string()));
    }

    #[test]
    fn user_owner() {
        let yaml = "spec:\n  owner: user:johndoe\n";
        assert_eq!(extract_owner(yaml), Some("johndoe".to_string()));
    }

    #[test]
    fn missing_spec() {
        let yaml = "apiVersion: backstage.io/v1alpha1\nkind: Component\n";
        assert_eq!(extract_owner(yaml), None);
    }

    #[test]
    fn missing_owner() {
        let yaml = "spec:\n  type: service\n";
        assert_eq!(extract_owner(yaml), None);
    }

    #[test]
    fn empty_content() {
        assert_eq!(extract_owner(""), None);
    }

    #[test]
    fn extra_fields_ignored() {
        let yaml = "spec:\n  owner: group:my-team\n  type: service\n  lifecycle: production\n";
        assert_eq!(extract_owner(yaml), Some("my-team".to_string()));
    }
}
