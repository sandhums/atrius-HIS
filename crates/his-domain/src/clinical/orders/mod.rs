//! CPOE order builders (Phase 5b).

mod lab_order;

pub use lab_order::{
    LabCatalogEntry, LAB_CATALOG, build_lab_diagnostic_report, build_lab_fulfillment_task,
    build_lab_result_observation, build_lab_service_request, is_lab_diagnostic_report,
    is_lab_fulfillment_task, is_lab_service_request, lab_fulfillment_task_id,
    lab_order_place_transaction, lab_result_observation_id, lab_result_report_id,
    lab_result_transaction, lab_service_request_id, resolve_lab_display,
};
