//! Watchdog Framework - Legacy module redirected to WatchdogAgent
//! This module now serves as a compatibility layer pointing to the new WatchdogAgent implementation

// We can't use pub use here since watchdog_agent is now a separate module
// So we re-export the types individually where needed

// Legacy re-exports for backward compatibility (will be deprecated)
pub mod condition {
    //! Condition evaluation module
    use crate::domain::TriggerCondition;
    use serde_json::Value;

    /// Condition evaluator
    pub struct ConditionEvaluator;

    impl ConditionEvaluator {
        /// Evaluate a trigger condition against a result value
        pub fn evaluate_condition(&self, condition: &TriggerCondition, result: &Value) -> bool {
            match condition {
                TriggerCondition::NumericRange { min, max } => {
                    if let Some(num_val) = result.as_f64() {
                        num_val >= *min && num_val <= *max
                    } else {
                        false
                    }
                },
                TriggerCondition::StringContains { content } => {
                    if let Some(str_val) = result.as_str() {
                        str_val.contains(content)
                    } else {
                        false
                    }
                },
                TriggerCondition::StatusMatches { expected_status } => {
                    if let Some(status_val) = result.as_str() {
                        status_val == expected_status
                    } else {
                        false
                    }
                },
                TriggerCondition::CustomExpression { .. } => {
                    // 简单实现，实际应用中可能需要更复杂的表达式解析
                    false
                },
            }
        }
    }
}


