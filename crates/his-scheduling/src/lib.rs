mod error;
mod service;

pub use error::SchedulingError;
pub use service::{
    BookAppointmentRequest, BookAppointmentResponse, CancelAppointmentRequest,
    FindSlotsQuery, FindSlotsResponse, RescheduleAppointmentRequest, SchedulingService,
    SlotSummary,
};
