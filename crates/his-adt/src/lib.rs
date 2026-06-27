mod error;
mod service;

pub use error::AdtError;
pub use service::{
    AdmitPatientRequest, AdmitPatientResponse, AdtService, BedBoardEntry, BedBoardQuery,
    BedBoardResponse, DischargePatientRequest, EncounterSummary, FinishVisitRequest,
    FinishVisitResponse, PractitionerEncountersQuery, PractitionerEncountersResponse,
    StartVisitRequest, StartVisitResponse, TransferPatientRequest,
};
