//! Watchdog条件评估器
//!
//! 提供复杂的条件评估能力

use serde_json::Value;

/// 条件评估器
pub struct ConditionEvaluator;

impl ConditionEvaluator {
    /// 评估触发条件
    pub fn evaluate_condition(&self, condition: &crate::core::watchdog::TriggerCondition, result: &Value) -> bool {
        match condition {
            crate::core::watchdog::TriggerCondition::NumericRange { min, max } => {
                self.evaluate_numeric_range(result, *min, *max)
            }
            crate::core::watchdog::TriggerCondition::StringContains { content } => {
                self.evaluate_string_contains(result, content)
            }
            crate::core::watchdog::TriggerCondition::StatusMatches { expected_status } => {
                self.evaluate_status_match(result, expected_status)
            }
            crate::core::watchdog::TriggerCondition::CustomExpression { expression } => {
                self.evaluate_custom_expression(result, expression)
            }
        }
    }

    /// 评估数值范围条件
    fn evaluate_numeric_range(&self, result: &Value, min: f64, max: f64) -> bool {
        if let Some(num_val) = result.as_f64() {
            return num_val >= min && num_val <= max;
        }

        // 如果结果是对象，尝试从中提取数值
        if let Some(obj) = result.as_object() {
            // 检查常见的数值字段名
            for field_name in ["value", "result", "data", "score", "count"] {
                if let Some(field_value) = obj.get(field_name) {
                    if let Some(num_val) = field_value.as_f64() {
                        if num_val >= min && num_val <= max {
                            return true;
                        }
                    }
                }
            }

            // 检查对象中的所有数值字段
            return obj.values()
                .any(|val| val.as_f64().map(|v| v >= min && v <= max).unwrap_or(false));
        }

        // 如果结果是数组，检查其中的数值
        if let Some(arr) = result.as_array() {
            return arr
                .iter()
                .any(|val| val.as_f64().map(|v| v >= min && v <= max).unwrap_or(false));
        }

        false
    }

    /// 评估字符串包含条件
    fn evaluate_string_contains(&self, result: &Value, content: &str) -> bool {
        if let Some(str_val) = result.as_str() {
            return str_val.contains(content);
        }

        // 如果不是字符串，转换为字符串形式再检查
        let result_str = result.to_string();
        result_str.contains(content)
    }

    /// 评估状态匹配条件
    fn evaluate_status_match(&self, result: &Value, expected_status: &str) -> bool {
        // 首先检查是否是字符串匹配
        if let Some(str_val) = result.as_str() {
            if str_val == expected_status {
                return true;
            }
        }

        // 如果是对象，检查常见的状态字段
        if let Some(obj) = result.as_object() {
            for field_name in ["status", "state", "result", "type"] {
                if let Some(field_value) = obj.get(field_name) {
                    if let Some(status_val) = field_value.as_str() {
                        if status_val == expected_status {
                            return true;
                        }
                    }
                }
            }
        }

        // 最后尝试将整个结果转换为字符串进行比较
        result.to_string() == *expected_status
    }

    /// 评估自定义表达式条件
    fn evaluate_custom_expression(&self, result: &Value, expression: &str) -> bool {
        // 这里我们实现一个简单的表达式评估器
        // 在实际应用中，可能需要集成专门的表达式解析库

        // 支持简单的变量替换和比较
        let expr_lower = expression.to_lowercase();

        // 检查是否包含比较操作符
        if expr_lower.contains(">=") {
            self.evaluate_comparison(result, ">=", &expr_lower)
        } else if expr_lower.contains("<=") {
            self.evaluate_comparison(result, "<=", &expr_lower)
        } else if expr_lower.contains('>') {
            self.evaluate_comparison(result, ">", &expr_lower)
        } else if expr_lower.contains('<') {
            self.evaluate_comparison(result, "<", &expr_lower)
        } else if expr_lower.contains("==") || expr_lower.contains('=') {
            self.evaluate_equality(result, &expr_lower)
        } else {
            // 如果没有比较操作符，当作字符串包含检查
            self.evaluate_string_contains(result, expression)
        }
    }

    /// 评估比较操作
    fn evaluate_comparison(&self, result: &Value, op: &str, expression: &str) -> bool {
        // 解析表达式 "value > 10" 或 "score >= 5.5"
        let parts: Vec<&str> = match op {
            ">=" => expression.splitn(2, ">=").collect(),
            "<=" => expression.splitn(2, "<=").collect(),
            ">" => expression.splitn(2, '>').collect(),
            "<" => expression.splitn(2, '<').collect(),
            _ => return false,
        };

        if parts.len() != 2 {
            return false;
        }

        let _field_name = parts[0].trim();
        let threshold_str = parts[1].trim();

        // 尝试解析阈值
        if let Ok(threshold) = threshold_str.parse::<f64>() {
            // 尝试从结果中提取数值进行比较
            if let Some(result_num) = self.extract_number_from_value(result) {
                return match op {
                    ">=" => result_num >= threshold,
                    "<=" => result_num <= threshold,
                    ">" => result_num > threshold,
                    "<" => result_num < threshold,
                    _ => false,
                };
            }
        }

        false
    }

    /// 评估相等性
    fn evaluate_equality(&self, result: &Value, expression: &str) -> bool {
        let parts: Vec<&str> = if expression.contains("==") {
            expression.splitn(2, "==").collect()
        } else {
            expression.splitn(2, '=').collect()
        };

        if parts.len() != 2 {
            return false;
        }

        let _field_name = parts[0].trim();
        let expected_value = parts[1].trim();

        // 尝试匹配期望值
        if let Some(str_val) = result.as_str() {
            return str_val == expected_value;
        }

        // 尝试作为数字比较
        if let Ok(expected_num) = expected_value.parse::<f64>() {
            if let Some(result_num) = self.extract_number_from_value(result) {
                return (result_num - expected_num).abs() < f64::EPSILON;
            }
        }

        false
    }

    /// 从值中提取数字
    fn extract_number_from_value(&self, value: &Value) -> Option<f64> {
        // 直接是数字的情况
        if let Some(num) = value.as_f64() {
            return Some(num);
        }

        // 如果是字符串，尝试解析为数字
        if let Some(str_val) = value.as_str() {
            if let Ok(num) = str_val.parse::<f64>() {
                return Some(num);
            }
        }

        // 如果是对象，尝试常见字段
        if let Some(obj) = value.as_object() {
            for field_name in ["value", "result", "data", "score", "count"] {
                if let Some(field_value) = obj.get(field_name) {
                    if let Some(num) = field_value.as_f64() {
                        return Some(num);
                    }
                }
            }
        }

        // 如果是数组，取第一个数字
        if let Some(arr) = value.as_array() {
            for item in arr {
                if let Some(num) = item.as_f64() {
                    return Some(num);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_numeric_range_evaluation() {
        let evaluator = ConditionEvaluator;

        // 测试直接数值
        assert!(evaluator.evaluate_condition(
            &crate::core::watchdog::TriggerCondition::NumericRange { min: 10.0, max: 20.0 },
            &json!(15.0)
        ));

        // 测试超出范围
        assert!(!evaluator.evaluate_condition(
            &crate::core::watchdog::TriggerCondition::NumericRange { min: 10.0, max: 20.0 },
            &json!(25.0)
        ));

        // 测试对象中的数值字段
        assert!(evaluator.evaluate_condition(
            &crate::core::watchdog::TriggerCondition::NumericRange { min: 10.0, max: 20.0 },
            &json!({"value": 15.0})
        ));
    }

    #[test]
    fn test_string_contains_evaluation() {
        let evaluator = ConditionEvaluator;

        assert!(evaluator.evaluate_condition(
            &crate::core::watchdog::TriggerCondition::StringContains { content: "success".to_string() },
            &json!("operation was successful")
        ));

        assert!(!evaluator.evaluate_condition(
            &crate::core::watchdog::TriggerCondition::StringContains { content: "success".to_string() },
            &json!("operation failed")
        ));
    }

    #[test]
    fn test_status_match_evaluation() {
        let evaluator = ConditionEvaluator;

        assert!(evaluator.evaluate_condition(
            &crate::core::watchdog::TriggerCondition::StatusMatches { expected_status: "success".to_string() },
            &json!("success")
        ));

        assert!(evaluator.evaluate_condition(
            &crate::core::watchdog::TriggerCondition::StatusMatches { expected_status: "success".to_string() },
            &json!({"status": "success"})
        ));
    }

    #[test]
    fn test_custom_expression_evaluation() {
        let evaluator = ConditionEvaluator;

        // 测试大于比较
        assert!(evaluator.evaluate_condition(
            &crate::core::watchdog::TriggerCondition::CustomExpression { expression: "value > 10".to_string() },
            &json!({"value": 15.0})
        ));

        // 测试等于比较
        assert!(evaluator.evaluate_condition(
            &crate::core::watchdog::TriggerCondition::CustomExpression { expression: "status = success".to_string() },
            &json!("success")
        ));
    }
}