#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_tracker_basic() {
        let tracker = DecisionTracker::new();
        
        // Test tracking decisions
        tracker.track_decision("test decision");
        tracker.track_decision("another decision");
        
        // Test updating available tools
        tracker.update_available_tools(vec!["tool1".to_string(), "tool2".to_string()]);
        
        // Test generating report
        let report = tracker.generate_transparency_report();
        assert_eq!(report.total_decisions, 2);
        assert_eq!(report.tool_calls, 2);
    }
}