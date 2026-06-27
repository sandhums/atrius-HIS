mod error;
mod service;

pub use error::OrderError;
pub use service::{
    LabCatalogResponse, LabOrderResponse, LabOrderSummary, LabResultSummary, LabTaskSummary,
    ListLabOrdersResponse, ListLabResultsResponse, ListLabTasksResponse, OrderService,
    PlaceLabOrderRequest, PostLabResultRequest, PostLabResultResponse, RevokeLabOrderResponse,
};
