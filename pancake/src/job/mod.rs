//! Cooperative shared stack-job orchestration.

mod lifecycle;

pub use lifecycle::{Generation, JobControl, JobSnapshot, JobState, Stage, Stop};
