mod error;
mod service;

pub use error::AdtError;
pub use service::{
    AdmitPatientRequest, AdmitPatientResponse, AdtService, BedBoardEntry, BedBoardQuery,
    BedBoardResponse, DischargePatientRequest, StartVisitRequest, StartVisitResponse,
    TransferPatientRequest,
};
