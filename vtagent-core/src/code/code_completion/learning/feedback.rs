/// Feedback processor for analyzing user interactions
pub struct FeedbackProcessor {
    feedback_history: Vec<FeedbackEntry>,
}

#[derive(Debug, Clone)]
struct FeedbackEntry {
    suggestion: String,
    accepted: bool,
    context: String,
    timestamp: std::time::SystemTime,
}

impl FeedbackProcessor {
    pub fn new() -> Self {
        Self {
            feedback_history: Vec::new(),
        }
    }

    /// Process user feedback
    pub fn process(&mut self, suggestion: &str, accepted: bool, context: &str) {
        let entry = FeedbackEntry {
            suggestion: suggestion.to_string(),
            accepted,
            context: context.to_string(),
            timestamp: std::time::SystemTime::now(),
        };

        self.feedback_history.push(entry);

        // Keep only recent feedback (last 1000 entries)
        if self.feedback_history.len() > 1000 {
            self.feedback_history.remove(0);
        }
    }

    /// Get acceptance rate for a specific pattern
    pub fn get_acceptance_rate(&self, pattern: &str) -> f64 {
        let relevant_feedback: Vec<_> = self
            .feedback_history
            .iter()
            .filter(|entry| entry.suggestion.contains(pattern))
            .collect();

        if relevant_feedback.is_empty() {
            return 0.5; // Default rate
        }

        let accepted_count = relevant_feedback
            .iter()
            .filter(|entry| entry.accepted)
            .count();
        accepted_count as f64 / relevant_feedback.len() as f64
    }
}
