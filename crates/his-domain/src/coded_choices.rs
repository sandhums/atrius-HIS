//! Static FHIR-aligned code lists for UI dropdowns and server-side request validation.

/// A selectable coded value (FHIR binding slice).
#[derive(Debug, Clone, Copy)]
pub struct CodedChoice {
    pub code: &'static str,
    pub display: &'static str,
    pub system: Option<&'static str>,
}

/// Choice groups exposed to registration UI and validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceGroup {
    Gender,
    TelecomSystem,
    TelecomUse,
    AddressUse,
}

pub const ADMINISTRATIVE_GENDER_SYSTEM: &str =
    "http://hl7.org/fhir/administrative-gender";
pub const CONTACT_POINT_SYSTEM: &str = "http://hl7.org/fhir/contact-point-system";
pub const CONTACT_POINT_USE: &str = "http://hl7.org/fhir/contact-point-use";
pub const ADDRESS_USE_SYSTEM: &str = "http://hl7.org/fhir/address-use";

const GENDER: &[CodedChoice] = &[
    CodedChoice {
        code: "male",
        display: "Male",
        system: Some(ADMINISTRATIVE_GENDER_SYSTEM),
    },
    CodedChoice {
        code: "female",
        display: "Female",
        system: Some(ADMINISTRATIVE_GENDER_SYSTEM),
    },
    CodedChoice {
        code: "other",
        display: "Other",
        system: Some(ADMINISTRATIVE_GENDER_SYSTEM),
    },
    CodedChoice {
        code: "unknown",
        display: "Unknown",
        system: Some(ADMINISTRATIVE_GENDER_SYSTEM),
    },
];

const TELECOM_SYSTEM: &[CodedChoice] = &[
    CodedChoice {
        code: "phone",
        display: "Phone",
        system: Some(CONTACT_POINT_SYSTEM),
    },
    CodedChoice {
        code: "email",
        display: "Email",
        system: Some(CONTACT_POINT_SYSTEM),
    },
    CodedChoice {
        code: "fax",
        display: "Fax",
        system: Some(CONTACT_POINT_SYSTEM),
    },
    CodedChoice {
        code: "pager",
        display: "Pager",
        system: Some(CONTACT_POINT_SYSTEM),
    },
    CodedChoice {
        code: "url",
        display: "URL",
        system: Some(CONTACT_POINT_SYSTEM),
    },
    CodedChoice {
        code: "sms",
        display: "SMS",
        system: Some(CONTACT_POINT_SYSTEM),
    },
    CodedChoice {
        code: "other",
        display: "Other",
        system: Some(CONTACT_POINT_SYSTEM),
    },
];

const TELECOM_USE: &[CodedChoice] = &[
    CodedChoice {
        code: "home",
        display: "Home",
        system: Some(CONTACT_POINT_USE),
    },
    CodedChoice {
        code: "work",
        display: "Work",
        system: Some(CONTACT_POINT_USE),
    },
    CodedChoice {
        code: "temp",
        display: "Temporary",
        system: Some(CONTACT_POINT_USE),
    },
    CodedChoice {
        code: "old",
        display: "Old / incorrect",
        system: Some(CONTACT_POINT_USE),
    },
    CodedChoice {
        code: "mobile",
        display: "Mobile",
        system: Some(CONTACT_POINT_USE),
    },
];

const ADDRESS_USE_CHOICES: &[CodedChoice] = &[
    CodedChoice {
        code: "home",
        display: "Home",
        system: Some(ADDRESS_USE_SYSTEM),
    },
    CodedChoice {
        code: "work",
        display: "Work",
        system: Some(ADDRESS_USE_SYSTEM),
    },
    CodedChoice {
        code: "temp",
        display: "Temporary",
        system: Some(ADDRESS_USE_SYSTEM),
    },
    CodedChoice {
        code: "old",
        display: "Old / incorrect",
        system: Some(ADDRESS_USE_SYSTEM),
    },
    CodedChoice {
        code: "billing",
        display: "Billing",
        system: Some(ADDRESS_USE_SYSTEM),
    },
];

/// All registration choice groups in API response order.
pub const REGISTRATION_CHOICE_GROUPS: &[(ChoiceGroup, &[CodedChoice])] = &[
    (ChoiceGroup::Gender, GENDER),
    (ChoiceGroup::TelecomSystem, TELECOM_SYSTEM),
    (ChoiceGroup::TelecomUse, TELECOM_USE),
    (ChoiceGroup::AddressUse, ADDRESS_USE_CHOICES),
];

#[must_use]
pub fn choices_for(group: ChoiceGroup) -> &'static [CodedChoice] {
    match group {
        ChoiceGroup::Gender => GENDER,
        ChoiceGroup::TelecomSystem => TELECOM_SYSTEM,
        ChoiceGroup::TelecomUse => TELECOM_USE,
        ChoiceGroup::AddressUse => ADDRESS_USE_CHOICES,
    }
}

#[must_use]
pub fn is_allowed_code(group: ChoiceGroup, code: &str) -> bool {
    choices_for(group)
        .iter()
        .any(|c| c.code == code)
}

#[must_use]
pub fn choice_display(group: ChoiceGroup, code: &str) -> Option<&'static str> {
    choices_for(group)
        .iter()
        .find(|c| c.code == code)
        .map(|c| c.display)
}

#[must_use]
pub fn group_key(group: ChoiceGroup) -> &'static str {
    match group {
        ChoiceGroup::Gender => "gender",
        ChoiceGroup::TelecomSystem => "telecom_system",
        ChoiceGroup::TelecomUse => "telecom_use",
        ChoiceGroup::AddressUse => "address_use",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gender_codes_match_administrative_gender() {
        assert!(is_allowed_code(ChoiceGroup::Gender, "female"));
        assert!(!is_allowed_code(ChoiceGroup::Gender, "invalid"));
    }

    #[test]
    fn telecom_system_rejects_unknown() {
        assert!(is_allowed_code(ChoiceGroup::TelecomSystem, "phone"));
        assert!(!is_allowed_code(ChoiceGroup::TelecomSystem, "telegram"));
    }
}
