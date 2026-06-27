mod error;
mod service;

pub use error::SchedulingError;
pub use service::{
    AppointmentSummary, BookAppointmentRequest, BookAppointmentResponse, BookingDoctorSummary,
    CancelAppointmentRequest, ExpandSlotsQuery, ExpandSlotsResponse, FindSlotsQuery,
    FindSlotsResponse, ListBookingDoctorsResponse, PractitionerAppointmentsQuery,
    PractitionerAppointmentsResponse, RescheduleAppointmentRequest, SchedulingService, SlotSummary,
};
