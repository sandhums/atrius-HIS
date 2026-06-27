//! SNOMED section slice metadata for value-sliced Composition profiles.

#[derive(Debug, Clone, Copy)]
pub enum EntryKind {
    Condition,
    Observation { exam: bool },
    AllergyIntolerance,
    ServiceRequest { category: &'static str, display: &'static str },
    MedicationStatement,
    MedicationRequest,
    DiagnosticReportLab,
    Appointment,
    Procedure,
    CarePlan,
}

#[derive(Debug, Clone)]
pub struct SnomedSliceDef<S> {
    pub slice: &'static str,
    pub title: &'static str,
    pub code: &'static str,
    pub display: &'static str,
    pub field: fn(&S) -> Option<&String>,
    pub entry: EntryKind,
    pub id_suffix: &'static str,
}

impl<S> SnomedSliceDef<S> {
    pub fn resource_type(&self) -> &'static str {
        match self.entry {
            EntryKind::Condition => "Condition",
            EntryKind::Observation { .. } => "Observation",
            EntryKind::AllergyIntolerance => "AllergyIntolerance",
            EntryKind::ServiceRequest { .. } => "ServiceRequest",
            EntryKind::MedicationStatement => "MedicationStatement",
            EntryKind::MedicationRequest => "MedicationRequest",
            EntryKind::DiagnosticReportLab => "DiagnosticReport",
            EntryKind::Appointment => "Appointment",
            EntryKind::Procedure => "Procedure",
            EntryKind::CarePlan => "CarePlan",
        }
    }
}
