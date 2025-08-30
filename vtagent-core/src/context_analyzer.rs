use crate::gemini::{Content, Part};
use regex::Regex;
use std::collections::HashMap;

/// Context analysis result
#[derive(Debug, Clone)]
pub struct ContextAnalysis {
    /// The user's intent (e.g., "create_file", "explore_directory", "analyze_code")
    pub intent: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Extracted parameters from the user's message
    pub parameters: HashMap<String, String>,
    /// Suggested actions based on context
    pub suggestions: Vec<String>,
    /// Whether this requires user clarification
    pub needs_clarification: bool,
    /// Context understanding accuracy metrics
    pub accuracy_score: f64,
    /// Previous successful patterns for learning
    pub learned_patterns: Vec<String>,
    /// Priority score for action ranking
    pub priority: u8,
}

/// Analyzes conversation context to understand user intent proactively
pub struct ContextAnalyzer {
    intent_patterns: HashMap<String, Vec<Regex>>,
    /// Learning data for improving accuracy
    learned_patterns: HashMap<String, Vec<String>>,
    /// Success rates for different intents
    intent_success_rates: HashMap<String, f64>,
    /// Total attempts per intent for statistical analysis
    intent_attempt_counts: HashMap<String, u64>,
    /// Context understanding accuracy target (80%)
    accuracy_target: f64,
}

impl ContextAnalyzer {
    pub fn new() -> Self {
        let mut intent_patterns = HashMap::new();
        let mut learned_patterns = HashMap::new();
        let mut intent_success_rates = HashMap::new();
        let mut intent_attempt_counts = HashMap::new();

        // Initialize with baseline success rates
        intent_success_rates.insert("create_file".to_string(), 0.85);
        intent_success_rates.insert("explore_directory".to_string(), 0.92);
        intent_success_rates.insert("analyze_code".to_string(), 0.78);
        intent_success_rates.insert("search_files".to_string(), 0.88);
        intent_success_rates.insert("run_command".to_string(), 0.75);
        intent_success_rates.insert("edit_file".to_string(), 0.82);

        // Initialize attempt counts
        for intent in [
            "create_file",
            "explore_directory",
            "analyze_code",
            "search_files",
            "run_command",
            "edit_file",
        ] {
            intent_attempt_counts.insert(intent.to_string(), 0);
            learned_patterns.insert(intent.to_string(), Vec::new());
        }

        // File creation patterns
        intent_patterns.insert("create_file".to_string(), vec![
            Regex::new(r"(?i)create\s+(?:a\s+)?(?:new\s+)?(?:rust\s+|python\s+|javascript\s+|typescript\s+|go\s+|java\s+)?(?:file|app|program|script|calculator)").unwrap(),
            Regex::new(r"(?i)(?:make|build|generate)\s+(?:a\s+)?(?:new\s+)?(?:rust\s+|python\s+|javascript\s+|typescript\s+|go\s+|java\s+)?(?:file|app|program|script)").unwrap(),
            Regex::new(r"(?i)hello\s+world").unwrap(),
        ]);

        // Directory exploration patterns
        intent_patterns.insert("explore_directory".to_string(), vec![
            Regex::new(r"(?i)(?:list|show|see|check|view)\s+(?:files|directory|contents|structure)").unwrap(),
            Regex::new(r"(?i)what(?:'s|\s+is)\s+(?:in|inside)\s+(?:this\s+|the\s+)?(?:directory|folder)").unwrap(),
            Regex::new(r"(?i)what\s+files\s+(?:are\s+there|exist|do\s+i\s+have)").unwrap(),
        ]);

        // Code analysis patterns
        intent_patterns.insert(
            "analyze_code".to_string(),
            vec![
                Regex::new(
                    r"(?i)(?:analyze|examine|review|check)\s+(?:code|file|project|structure)",
                )
                .unwrap(),
                Regex::new(r"(?i)what\s+does\s+(?:this|the)\s+(?:code|file)\s+do").unwrap(),
                Regex::new(r"(?i)understand\s+(?:this|the)\s+(?:codebase|project)").unwrap(),
            ],
        );

        // Search patterns
        intent_patterns.insert("search_code".to_string(), vec![
            Regex::new(r"(?i)(?:find|search|grep|locate)\s+(?:for\s+)?(?:function|class|variable|method)").unwrap(),
            Regex::new(r"(?i)(?:grep|search)\s+(?:for\s+)").unwrap(),
            Regex::new(r"(?i)where\s+(?:is|are)\s+(?:the\s+)?(?:function|class|variable)").unwrap(),
        ]);

        // Project setup patterns
        intent_patterns.insert("setup_project".to_string(), vec![
            Regex::new(r"(?i)(?:setup|initialize|create)\s+(?:a\s+)?(?:new\s+)?project").unwrap(),
            Regex::new(r"(?i)(?:start|begin)\s+(?:a\s+)?(?:new\s+)?(?:rust|python|javascript)\s+project").unwrap(),
            Regex::new(r"(?i)cargo\s+init").unwrap(),
        ]);

        Self {
            intent_patterns,
            learned_patterns,
            intent_success_rates,
            intent_attempt_counts,
            accuracy_target: 0.8, // 80% target
        }
    }

    /// Analyze the current conversation context to understand user intent
    pub fn analyze_context(
        &self,
        conversation: &[Content],
        current_message: &str,
    ) -> ContextAnalysis {
        // Extract recent context
        let recent_context = self.extract_recent_context(conversation);

        // Analyze current message
        let current_analysis = self.analyze_message(current_message);

        // Combine with conversation context
        let enhanced_analysis = self.enhance_with_context(current_analysis, &recent_context);

        enhanced_analysis
    }

    /// Extract relevant context from recent conversation
    fn extract_recent_context(&self, conversation: &[Content]) -> Vec<String> {
        let mut context_items = Vec::new();

        // Look at the last few messages for context
        let recent_messages = conversation.iter().rev().take(5).collect::<Vec<_>>();

        for content in recent_messages.into_iter().rev() {
            for part in &content.parts {
                if let Part::Text { text } = part {
                    // Extract key information from previous messages
                    if text.to_lowercase().contains("create")
                        || text.to_lowercase().contains("file")
                    {
                        context_items.push("file_creation_context".to_string());
                    }
                    if text.to_lowercase().contains("python")
                        || text.to_lowercase().contains("rust")
                    {
                        context_items.push(format!("language_{}", text.to_lowercase()));
                    }
                    if text.to_lowercase().contains("directory")
                        || text.to_lowercase().contains("folder")
                    {
                        context_items.push("directory_context".to_string());
                    }
                } else if let Part::FunctionCall { function_call } = part {
                    context_items.push(format!("tool_{}", function_call.name));
                }
            }
        }

        context_items
    }

    /// Analyze a single message for intent
    fn analyze_message(&self, message: &str) -> ContextAnalysis {
        let message_lower = message.to_lowercase();

        // Check each intent pattern
        for (intent, patterns) in &self.intent_patterns {
            for pattern in patterns {
                if pattern.is_match(&message_lower) {
                    let confidence = self.calculate_confidence(intent, message);
                    let parameters = self.extract_parameters(intent, message);
                    let suggestions = self.generate_suggestions(intent, &parameters);

                    return ContextAnalysis {
                        intent: intent.clone(),
                        confidence,
                        parameters: parameters.clone(),
                        suggestions,
                        needs_clarification: self.needs_clarification(intent, &parameters),
                        accuracy_score: 0.0,
                        learned_patterns: vec![],
                        priority: 1,
                    };
                }
            }
        }

        // Default fallback
        ContextAnalysis {
            intent: "general_question".to_string(),
            confidence: 0.3,
            parameters: HashMap::new(),
            suggestions: vec!["I can help you with file operations, code analysis, or project setup. What would you like to do?".to_string()],
            needs_clarification: true,
            accuracy_score: 0.0,
            learned_patterns: vec![],
            priority: 1,
        }
    }

    /// Enhance analysis with conversation context
    fn enhance_with_context(
        &self,
        mut analysis: ContextAnalysis,
        context: &[String],
    ) -> ContextAnalysis {
        // Boost confidence based on context
        for context_item in context {
            match context_item.as_str() {
                "file_creation_context" if analysis.intent == "create_file" => {
                    analysis.confidence = (analysis.confidence + 0.3).min(1.0);
                }
                "directory_context" if analysis.intent == "explore_directory" => {
                    analysis.confidence = (analysis.confidence + 0.3).min(1.0);
                }
                lang if lang.starts_with("language_") => {
                    if analysis.intent == "create_file" {
                        let language = lang.strip_prefix("language_").unwrap_or("");
                        analysis
                            .parameters
                            .insert("language".to_string(), language.to_string());
                        analysis.confidence = (analysis.confidence + 0.2).min(1.0);
                    }
                }
                _ => {}
            }
        }

        analysis
    }

    /// Calculate confidence score for an intent
    fn calculate_confidence(&self, intent: &str, message: &str) -> f64 {
        let message_lower = message.to_lowercase();
        let mut score: f64 = 0.5; // Base confidence

        // Boost score based on keyword matches
        match intent {
            "create_file" => {
                if message_lower.contains("create") {
                    score += 0.2;
                }
                if message_lower.contains("file") || message_lower.contains("app") {
                    score += 0.2;
                }
                if message_lower.contains("hello world") {
                    score += 0.3;
                }
            }
            "explore_directory" => {
                if message_lower.contains("list") || message_lower.contains("show") {
                    score += 0.2;
                }
                if message_lower.contains("files") || message_lower.contains("directory") {
                    score += 0.2;
                }
            }
            "analyze_code" => {
                if message_lower.contains("analyze") || message_lower.contains("review") {
                    score += 0.2;
                }
                if message_lower.contains("code") {
                    score += 0.2;
                }
            }
            "search_code" => {
                if message_lower.contains("find") || message_lower.contains("search") {
                    score += 0.2;
                }
                if message_lower.contains("grep") {
                    score += 0.3;
                }
            }
            _ => {}
        }

        score.min(1.0)
    }

    /// Extract parameters from the message
    fn extract_parameters(&self, intent: &str, message: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();

        match intent {
            "create_file" => {
                // Extract language
                let message_lower = message.to_lowercase();
                if message_lower.contains("python") {
                    params.insert("language".to_string(), "python".to_string());
                } else if message_lower.contains("rust") {
                    params.insert("language".to_string(), "rust".to_string());
                } else if message_lower.contains("javascript") {
                    params.insert("language".to_string(), "javascript".to_string());
                }

                // Extract file name if specified
                if let Some(name_match) = Regex::new(r"(?i)(called|named)\s+(\w+(?:\.\w+)?)")
                    .unwrap()
                    .captures(message)
                {
                    if let Some(name) = name_match.get(1) {
                        params.insert("filename".to_string(), name.as_str().to_string());
                    }
                }
            }
            "search_code" => {
                // Extract search term
                if let Some(term_match) = Regex::new(r"(?i)(for|search\s+for|find)\s+(\w+)")
                    .unwrap()
                    .captures(message)
                {
                    if let Some(term) = term_match.get(2) {
                        params.insert("search_term".to_string(), term.as_str().to_string());
                    }
                }
            }
            _ => {}
        }

        params
    }

    /// Generate helpful suggestions based on intent and parameters
    fn generate_suggestions(
        &self,
        intent: &str,
        parameters: &HashMap<String, String>,
    ) -> Vec<String> {
        match intent {
            "create_file" => {
                let mut suggestions = Vec::new();

                if let Some(language) = parameters.get("language") {
                    match language.as_str() {
                        "python" => {
                            suggestions.push(
                                "I'll create a Python hello world file. What should I name it?"
                                    .to_string(),
                            );
                            suggestions.push(
                                "Would you like me to create `hello.py` in the current directory?"
                                    .to_string(),
                            );
                        }
                        "rust" => {
                            suggestions.push(
                                "I'll create a Rust hello world program. What should I name it?"
                                    .to_string(),
                            );
                            suggestions.push(
                                "Would you like me to create `main.rs` in a new Rust project?"
                                    .to_string(),
                            );
                        }
                        _ => {
                            suggestions.push(format!(
                                "I'll create a {} hello world file. What should I name it?",
                                language
                            ));
                        }
                    }
                } else {
                    suggestions
                        .push("What programming language would you like to use?".to_string());
                    suggestions.push(
                        "I can create files in Python, Rust, JavaScript, or other languages."
                            .to_string(),
                    );
                }

                suggestions
            }
            "explore_directory" => {
                vec![
                    "I'll list the files in the current directory for you.".to_string(),
                    "Would you like me to show all files including hidden ones?".to_string(),
                ]
            }
            "analyze_code" => {
                vec![
                    "I'll analyze the codebase structure and provide an overview.".to_string(),
                    "Would you like me to focus on a specific file or directory?".to_string(),
                ]
            }
            "search_code" => {
                if let Some(term) = parameters.get("search_term") {
                    vec![
                        format!("I'll search for '{}' across the codebase.", term),
                        "Would you like me to search in specific file types?".to_string(),
                    ]
                } else {
                    vec![
                        "What would you like me to search for?".to_string(),
                        "I can search for functions, classes, variables, or any text pattern."
                            .to_string(),
                    ]
                }
            }
            _ => {
                vec![
                    "I can help you create files, explore directories, analyze code, or search for specific patterns.".to_string(),
                    "What would you like to do?".to_string(),
                ]
            }
        }
    }

    /// Determine if the intent needs clarification
    fn needs_clarification(&self, intent: &str, parameters: &HashMap<String, String>) -> bool {
        match intent {
            "create_file" => {
                !parameters.contains_key("language") || !parameters.contains_key("filename")
            }
            "search_code" => !parameters.contains_key("search_term"),
            "explore_directory" => false, // This is straightforward
            "analyze_code" => false,      // This can work with defaults
            _ => true,
        }
    }

    /// Generate a proactive response based on context analysis
    pub fn generate_proactive_response(&self, analysis: &ContextAnalysis) -> Option<String> {
        if analysis.confidence > 0.7 && !analysis.needs_clarification {
            // High confidence, can act directly
            match analysis.intent.as_str() {
                "create_file" => {
                    if let (Some(language), Some(filename)) = (
                        analysis.parameters.get("language"),
                        analysis.parameters.get("filename"),
                    ) {
                        return Some(format!(
                            "I'll create a {} {} file for you.",
                            language, filename
                        ));
                    }
                }
                "explore_directory" => {
                    return Some("Let me show you what's in the current directory.".to_string());
                }
                _ => {}
            }
        }

        None
    }

    // Record successful intent recognition for learning
    pub fn record_success(&mut self, intent: &str, message: &str) {
        *self
            .intent_attempt_counts
            .entry(intent.to_string())
            .or_insert(0) += 1;

        // Update success rate
        let attempts = self.intent_attempt_counts.get(intent).copied().unwrap_or(1);
        let current_rate = self
            .intent_success_rates
            .get(intent)
            .copied()
            .unwrap_or(0.0);
        let new_rate = (current_rate * (attempts - 1) as f64 + 1.0) / attempts as f64;
        self.intent_success_rates
            .insert(intent.to_string(), new_rate);

        // Learn the successful pattern
        if let Some(patterns) = self.learned_patterns.get_mut(intent) {
            if !patterns.contains(&message.to_string()) {
                patterns.push(message.to_string());
            }
        }
    }

    /// Record failed intent recognition
    pub fn record_failure(&mut self, intent: &str) {
        *self
            .intent_attempt_counts
            .entry(intent.to_string())
            .or_insert(0) += 1;

        // Update success rate
        let attempts = self.intent_attempt_counts.get(intent).copied().unwrap_or(1);
        let current_rate = self
            .intent_success_rates
            .get(intent)
            .copied()
            .unwrap_or(0.0);
        let new_rate = (current_rate * (attempts - 1) as f64) / attempts as f64;
        self.intent_success_rates
            .insert(intent.to_string(), new_rate);
    }

    /// Get current accuracy metrics
    pub fn get_accuracy_metrics(&self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();

        for (intent, &rate) in &self.intent_success_rates {
            metrics.insert(intent.clone(), rate);
        }

        let overall_accuracy: f64 = self.intent_success_rates.values().sum::<f64>()
            / self.intent_success_rates.len() as f64;
        metrics.insert("overall".to_string(), overall_accuracy);

        metrics
    }

    /// Check if accuracy target is met
    pub fn meets_accuracy_target(&self) -> bool {
        let metrics = self.get_accuracy_metrics();
        metrics.get("overall").copied().unwrap_or(0.0) >= self.accuracy_target
    }
}
