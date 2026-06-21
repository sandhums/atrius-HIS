use uuid::Uuid;

/// Generate a hospital MRN value (not yet persisted — uniqueness checked at registration).
#[must_use]
pub fn generate_mrn() -> String {
    let id = Uuid::new_v4().simple().to_string();
    format!("MRN-{}", &id[..12].to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mrn_has_expected_prefix() {
        let mrn = generate_mrn();
        assert!(mrn.starts_with("MRN-"));
        assert!(mrn.len() > 8);
    }
}
